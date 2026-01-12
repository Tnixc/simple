#include "minify.hpp"
#include <cctype>

namespace simple {

// Simple CSS minification
static auto minify_css(std::string_view css) -> std::string {
  std::string result;
  result.reserve(css.size());

  bool in_string = false;
  char string_char = 0;
  bool in_comment = false;
  bool last_was_space = false;

  for (size_t i = 0; i < css.size(); ++i) {
    char ch = css[i];

    // Handle CSS comments /* */
    if (!in_string && !in_comment && i + 1 < css.size() && ch == '/' &&
        css[i + 1] == '*') {
      in_comment = true;
      ++i;
      continue;
    }

    if (in_comment) {
      if (i + 1 < css.size() && ch == '*' && css[i + 1] == '/') {
        in_comment = false;
        ++i;
      }
      continue;
    }

    // Handle strings
    if (!in_string && (ch == '"' || ch == '\'')) {
      in_string = true;
      string_char = ch;
      result += ch;
      last_was_space = false;
      continue;
    }

    if (in_string) {
      result += ch;
      if (ch == string_char && (i == 0 || css[i - 1] != '\\')) {
        in_string = false;
      }
      continue;
    }

    // Collapse whitespace
    if (std::isspace(static_cast<unsigned char>(ch)) != 0) {
      // Only emit space if needed (not after certain chars)
      if (!last_was_space && !result.empty()) {
        char last = result.back();
        // Don't need space after these
        if (last != '{' && last != '}' && last != ';' && last != ':' &&
            last != ',' && last != '>' && last != '+' && last != '~' &&
            last != '(' && last != ')') {
          last_was_space = true;
        }
      }
      continue;
    }

    // Remove space before certain characters
    if (last_was_space) {
      if (ch != '{' && ch != '}' && ch != ';' && ch != ':' && ch != ',' &&
          ch != '>' && ch != '+' && ch != '~' && ch != '(' && ch != ')') {
        result += ' ';
      }
      last_was_space = false;
    }

    result += ch;
  }

  return result;
}

auto minify_html(std::string_view html) -> std::string {
  std::string result;
  result.reserve(html.size());

  bool in_tag = false;
  bool in_comment = false;
  bool in_script = false;
  bool in_style = false;
  bool in_pre = false;
  bool in_textarea = false;
  bool preserve_whitespace = false;
  bool last_was_whitespace = false;
  bool in_tag_whitespace = false;

  std::string style_content;
  bool collecting_style = false;

  for (size_t i = 0; i < html.size(); ++i) {
    char ch = html[i];

    if (!in_comment && i + 4 < html.size() && html.substr(i, 4) == "<!--") {
      in_comment = true;
      i += 3;
      continue;
    }

    if (in_comment) {
      if (i + 3 <= html.size() && html.substr(i, 3) == "-->") {
        in_comment = false;
        i += 2;
      }
      continue;
    }

    if (ch == '<') {
      in_tag = true;
      in_tag_whitespace = false;

      if (i + 7 < html.size() && html.substr(i, 7) == "<script") {
        in_script = true;
        preserve_whitespace = true;
      } else if (i + 6 < html.size() && html.substr(i, 6) == "<style") {
        in_style = true;
        collecting_style = false; // Start collecting after >
      } else if (i + 4 < html.size() && html.substr(i, 4) == "<pre") {
        in_pre = true;
        preserve_whitespace = true;
      } else if (i + 9 < html.size() && html.substr(i, 9) == "<textarea") {
        in_textarea = true;
        preserve_whitespace = true;
      } else if (i + 9 < html.size() && html.substr(i, 9) == "</script>") {
        in_script = false;
        preserve_whitespace = in_pre || in_textarea;
      } else if (i + 8 < html.size() && html.substr(i, 8) == "</style>") {
        // Minify and output collected CSS
        if (!style_content.empty()) {
          result += minify_css(style_content);
          style_content.clear();
        }
        in_style = false;
        collecting_style = false;
        preserve_whitespace = in_script || in_pre || in_textarea;
      } else if (i + 6 < html.size() && html.substr(i, 6) == "</pre>") {
        in_pre = false;
        preserve_whitespace = in_script || in_style || in_textarea;
      } else if (i + 11 < html.size() && html.substr(i, 11) == "</textarea>") {
        in_textarea = false;
        preserve_whitespace = in_script || in_style || in_pre;
      }

      // Add pending whitespace before tag if there was content before
      if (last_was_whitespace && !result.empty() && result.back() != '>') {
        result += ' ';
      }
      last_was_whitespace = false;

      result += ch;
      continue;
    }

    if (ch == '>') {
      // Remove trailing space before >
      if (!result.empty() && result.back() == ' ') {
        result.pop_back();
      }
      in_tag = false;
      in_tag_whitespace = false;
      last_was_whitespace = false;
      result += ch;

      // Start collecting style content after style tag closes
      if (in_style && !collecting_style) {
        collecting_style = true;
        style_content.clear();
      }
      continue;
    }

    // Inside a tag - collapse whitespace to single space
    if (in_tag) {
      if (std::isspace(static_cast<unsigned char>(ch)) != 0) {
        if (!in_tag_whitespace) {
          result += ' ';
          in_tag_whitespace = true;
        }
      } else {
        in_tag_whitespace = false;
        result += ch;
      }
      continue;
    }

    // Collect style content for later minification
    if (collecting_style) {
      style_content += ch;
      continue;
    }

    if (preserve_whitespace) {
      result += ch;
      continue;
    }

    if (std::isspace(static_cast<unsigned char>(ch)) != 0) {
      // Track that we saw whitespace - we may need to emit it
      last_was_whitespace = true;
      continue;
    }

    // Non-whitespace character outside tag
    // If there was whitespace and there was previous content, add a single
    // space
    if (last_was_whitespace && !result.empty()) {
      result += ' ';
    }
    last_was_whitespace = false;

    result += ch;
  }

  while (!result.empty() &&
         std::isspace(static_cast<unsigned char>(result.back())) != 0) {
    result.pop_back();
  }

  return result;
}

} // namespace simple
