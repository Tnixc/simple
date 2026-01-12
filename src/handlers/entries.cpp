#include "handlers/entries.hpp"
#include "handlers/frontmatter.hpp"
#include "handlers/katex_assets.hpp"
#include "handlers/pages.hpp"
#include "minify.hpp"
#include "utils.hpp"
#include <fstream>
#include <sstream>

namespace simple::handlers {

extern bool is_dev;
extern uint16_t ws_port;
extern std::string_view script;

auto process_entry(
    const std::filesystem::path &src, std::string_view name,
    std::string entry_path, std::string result_path,
    std::vector<std::pair<std::string_view, std::string_view>> kv)
    -> std::vector<ProcessError> {

  std::vector<ProcessError> errors;

  reset_katex_flag();

  if (entry_path.empty() || result_path.empty()) {
    return {ProcessError{
        ErrorType::Other, WithItem::Template, result_path,
        std::format("Error occurred in {}. The --entry-path and --result-path "
                    "keys must both be present if either is present.",
                    name)}};
  }

  if (entry_path.starts_with("/")) {
    entry_path = entry_path.substr(1);
  }

  auto entry_file_path = src / "data" / entry_path;

  std::string name_str{name};
  size_t pos = 0;
  while ((pos = name_str.find(':', pos)) != std::string::npos) {
    name_str.replace(pos, 1, "/");
  }

  auto frame_path = src / "templates" / (name_str + ".frame.html");

  auto src_parent = src.parent_path();
  if (src_parent.empty()) {
    return {ProcessError{ErrorType::Io, WithItem::File, src,
                         "Source directory has no parent"}};
  }

  if (result_path.starts_with("/")) {
    result_path = result_path.substr(1);
  }

  auto result_file_path = src_parent / (is_dev ? "dev" : "dist") / result_path;

  std::ifstream frame_file{frame_path};
  if (!frame_file) {
    return {ProcessError{ErrorType::Io, WithItem::Data, frame_path,
                         "Failed to read frame file"}};
  }

  std::stringstream frame_buffer;
  frame_buffer << frame_file.rdbuf();
  auto frame_content = frame_buffer.str();

  std::ifstream entry_file{entry_file_path};
  if (!entry_file) {
    return {ProcessError{ErrorType::Io, WithItem::Data, entry_file_path,
                         "Failed to read data file"}};
  }

  std::stringstream entry_buffer;
  entry_buffer << entry_file.rdbuf();
  auto content = entry_buffer.str();

  std::string processed_content;

  if (entry_file_path.extension() == ".md") {
    auto fm_result = extract_frontmatter(content);
    std::string content_without_frontmatter;

    if (fm_result) {
      content_without_frontmatter = std::move(fm_result->second);
    } else {
      content_without_frontmatter = content;
    }

    processed_content = frame_content;
    auto markdown_wrapped =
        "<markdown>\n" + content_without_frontmatter + "</markdown>";
    size_t pos = processed_content.find("${--content}");
    if (pos != std::string::npos) {
      processed_content.replace(pos, 12, markdown_wrapped);
    }
  } else {
    processed_content = frame_content;
    size_t pos = processed_content.find("${--content}");
    if (pos != std::string::npos) {
      processed_content.replace(pos, 12, content);
    }
  }

  auto final_content = kv_replace(kv, std::move(processed_content));
  auto page_result = page(src, std::move(final_content), {});

  errors.insert(errors.end(), page_result.errors.begin(),
                page_result.errors.end());

  if (result_file_path.has_parent_path()) {
    std::error_code ec;
    std::filesystem::create_directories(result_file_path.parent_path(), ec);
    if (ec) {
      errors.push_back(ProcessError{
          ErrorType::Io, WithItem::File, result_file_path.parent_path(),
          std::format("Failed to create directory structure: {}",
                      ec.message())});
    }
  }

  auto s = std::move(page_result.output);

  if (was_katex_used() && !is_katex_injection_disabled()) {
    print_katex_message();

    auto katex_css = get_katex_css_tag();
    if (s.find("<head>") != std::string::npos) {
      auto head_tag = std::format("<head>\n{}", katex_css);
      size_t pos = s.find("<head>");
      s.replace(pos, 6, head_tag);
    } else {
      s = std::format("{}\n{}", katex_css, s);
    }
  }

  if (is_dev &&
      s.find("// * SCRIPT INCLUDED IN DEV MODE") == std::string::npos) {
    auto head_with_script = std::format("<head>{}", script);
    size_t pos = s.find("<head>");
    if (pos != std::string::npos) {
      s.replace(pos, 6, head_with_script);
    }

    size_t ws_pos = 0;
    while ((ws_pos = s.find("__SIMPLE_WS_PORT_PLACEHOLDER__", ws_pos)) !=
           std::string::npos) {
      s.replace(ws_pos, 30, std::to_string(ws_port));
      ws_pos += std::to_string(ws_port).length();
    }
  }

  std::string to_write;
  if (is_dev) {
    to_write = std::move(s);
  } else {
    to_write = minify_html(s);
  }

  std::ofstream output_file{result_file_path, std::ios::binary};
  if (!output_file) {
    errors.push_back(ProcessError{ErrorType::Io, WithItem::File,
                                  result_file_path,
                                  "Failed to write result file"});
    return errors;
  }

  output_file << to_write;

  return errors;
}

} // namespace simple::handlers
