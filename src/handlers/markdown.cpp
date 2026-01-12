#include "handlers/markdown.hpp"
#include "utils.hpp"
#include <cmark-gfm-core-extensions.h>
#include <cmark-gfm-extension_api.h>
#include <cmark-gfm.h>
#include <mutex>
#include <regex>

namespace simple::handlers {

static const std::regex markdown_regex{R"(<markdown>([\s\S]+?)<\/markdown>)"};
static const std::regex highlight_regex{R"(==([^=]+)==)"};

extern bool is_dev;

static std::once_flag extensions_init_flag;

static void ensure_extensions_registered() {
  std::call_once(extensions_init_flag,
                 []() { cmark_gfm_core_extensions_ensure_registered(); });
}

static auto trim(std::string_view s) -> std::string_view {
  auto start = s.find_first_not_of(" \t\n\r");
  if (start == std::string_view::npos)
    return {};
  auto end = s.find_last_not_of(" \t\n\r");
  return s.substr(start, end - start + 1);
}

// Process ==highlight== syntax to <mark>highlight</mark>
static auto process_highlights(const std::string &html) -> std::string {
  return std::regex_replace(html, highlight_regex, "<mark>$1</mark>");
}

static auto render_markdown_content(const std::string &markdown)
    -> std::string {
  // Ensure extensions are registered (thread-safe)
  ensure_extensions_registered();

  // Create parser with all options matching comrak
  int options = CMARK_OPT_DEFAULT | CMARK_OPT_UNSAFE | CMARK_OPT_FOOTNOTES;
  cmark_parser *parser = cmark_parser_new(options);

  // Attach GFM extensions (table, strikethrough, autolink, tasklist)
  cmark_syntax_extension *table_ext = cmark_find_syntax_extension("table");
  cmark_syntax_extension *strikethrough_ext =
      cmark_find_syntax_extension("strikethrough");
  cmark_syntax_extension *autolink_ext =
      cmark_find_syntax_extension("autolink");
  cmark_syntax_extension *tasklist_ext =
      cmark_find_syntax_extension("tasklist");

  if (table_ext)
    cmark_parser_attach_syntax_extension(parser, table_ext);
  if (strikethrough_ext)
    cmark_parser_attach_syntax_extension(parser, strikethrough_ext);
  if (autolink_ext)
    cmark_parser_attach_syntax_extension(parser, autolink_ext);
  if (tasklist_ext)
    cmark_parser_attach_syntax_extension(parser, tasklist_ext);

  // Parse
  cmark_parser_feed(parser, markdown.c_str(), markdown.size());
  cmark_node *doc = cmark_parser_finish(parser);

  // Get extensions list for rendering
  cmark_llist *extensions = cmark_parser_get_syntax_extensions(parser);

  // Render to HTML
  char *html_c = cmark_render_html(doc, options, extensions);
  std::string html{html_c};

  free(html_c);
  cmark_node_free(doc);
  cmark_parser_free(parser);

  // Process ==highlight== syntax
  html = process_highlights(html);

  return html;
}

auto render_markdown(std::string input) -> std::string {
  if (input.find("</markdown>") == std::string::npos) {
    return input;
  }

  std::string result;
  result.reserve(input.size() + input.size() / 4);

  auto begin = std::sregex_iterator(input.begin(), input.end(), markdown_regex);
  auto end = std::sregex_iterator();
  size_t last_end = 0;

  for (auto it = begin; it != end; ++it) {
    std::smatch match = *it;

    if (is_inside_comment(input, match.position())) {
      continue;
    }

    size_t match_start = match.position();
    auto markdown_content = match[1].str();

    result.append(input, last_end, match_start - last_end);

    auto unindented = unindent(markdown_content);
    auto rendered = render_markdown_content(unindented);

    if (is_dev) {
      result += R"(<div style='display: contents;' data-markdown-source=")";

      // Trim the unindented content before escaping (matches Rust behavior)
      auto trimmed = trim(unindented);
      for (char ch : trimmed) {
        switch (ch) {
        case '"':
          result += "&quot;";
          break;
        case '&':
          result += "&amp;";
          break;
        case '<':
          result += "&lt;";
          break;
        case '>':
          result += "&gt;";
          break;
        default:
          result += ch;
        }
      }

      result += R"(">)";
      result += rendered;
      result += "</div>";
    } else {
      result += R"(<div style='display: contents;'>)";
      result += rendered;
      result += "</div>";
    }

    last_end = match.position() + match.length();
  }

  result.append(input, last_end);
  return result;
}

} // namespace simple::handlers
