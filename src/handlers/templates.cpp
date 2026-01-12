#include "handlers/templates.hpp"
#include "handlers/entries.hpp"
#include "handlers/frontmatter.hpp"
#include "handlers/pages.hpp"
#include <fstream>
#include <nlohmann/json.hpp>
#include <regex>
#include <sstream>

namespace simple::handlers {

static const std::regex template_regex{
    R"(<\::Template\{([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)\}\s*\/>)"};

auto get_template(const std::filesystem::path &src, std::string_view name,
                  std::unordered_set<std::filesystem::path> hist)
    -> ProcessResult {

  std::vector<ProcessError> errors;

  std::string name_str{name};
  size_t pos = 0;
  while ((pos = name_str.find(':', pos)) != std::string::npos) {
    name_str.replace(pos, 1, "/");
  }

  auto template_path = src / "templates" / (name_str + ".template.html");
  auto data_path = src / "data" / (name_str + ".data.json");

  if (!hist.insert(template_path).second) {
    return ProcessResult{
        "",
        {ProcessError{ErrorType::Circular, WithItem::Template, template_path,
                      "Circular dependency detected"}}};
  }

  std::ifstream template_file{template_path};
  if (!template_file) {
    return ProcessResult{
        "",
        {ProcessError{ErrorType::Io, WithItem::Template, template_path,
                      "Failed to read template file"}}};
  }

  std::stringstream template_buffer;
  template_buffer << template_file.rdbuf();
  auto template_str = template_buffer.str();

  if (template_str.empty()) {
    return ProcessResult{"", errors};
  }

  auto toml_path = src / "data" / (name_str + ".data.toml");
  nlohmann::json v;

  if (std::filesystem::exists(toml_path)) {
    auto fm_result = load_frontmatter_data(src, name);
    if (fm_result) {
      auto [value, fm_errors] = std::move(*fm_result);
      v = std::move(value);
      errors.insert(errors.end(), fm_errors.begin(), fm_errors.end());
    } else {
      errors.insert(errors.end(), fm_result.error().begin(),
                    fm_result.error().end());
      return ProcessResult{"", errors};
    }
  } else {
    std::ifstream data_file{data_path};
    if (!data_file) {
      return ProcessResult{
          "",
          {ProcessError{ErrorType::Io, WithItem::Data, data_path,
                        "Failed to read data file"}}};
    }

    std::stringstream data_buffer;
    data_buffer << data_file.rdbuf();
    auto data_str = data_buffer.str();

    try {
      v = nlohmann::json::parse(data_str);
    } catch (const nlohmann::json::exception &e) {
      return ProcessResult{
          "",
          {ProcessError{ErrorType::Syntax, WithItem::Data, data_path,
                        std::format("JSON decode error: {}", e.what())}}};
    }
  }

  if (!v.is_array()) {
    return ProcessResult{"",
                         {ProcessError{ErrorType::Syntax, WithItem::Data,
                                       data_path, "JSON wasn't an array"}}};
  }

  std::string contents;
  contents.reserve(template_str.size() * v.size());

  for (const auto &object : v) {
    if (!object.is_object()) {
      errors.push_back(ProcessError{ErrorType::Syntax, WithItem::Data,
                                    data_path, "Invalid object in JSON"});
      continue;
    }

    std::string entry_path;
    std::string result_path;
    bool is_entry = false;
    std::vector<std::pair<std::string, std::string>> kv_storage;
    std::vector<std::pair<std::string_view, std::string_view>> kv;

    for (auto it = object.begin(); it != object.end(); ++it) {
      if (!it.value().is_string()) {
        errors.push_back(
            ProcessError{ErrorType::Syntax, WithItem::Data, data_path,
                         "JSON object value couldn't be decoded to string"});
        continue;
      }

      auto key = it.key();
      auto val = it.value().get<std::string>();

      if (key == "--entry-path") {
        entry_path = val;
        is_entry = true;
      } else if (key == "--result-path") {
        result_path = val;
        is_entry = true;
      }

      kv_storage.emplace_back(key, val);
    }

    for (const auto &[k, v] : kv_storage) {
      kv.emplace_back(k, v);
    }

    auto processed_template = kv_replace(kv, template_str);
    contents += processed_template;

    if (is_entry) {
      auto entry_errs = process_entry(src, name, std::move(entry_path),
                                      std::move(result_path), kv);
      errors.insert(errors.end(), entry_errs.begin(), entry_errs.end());
    }
  }

  auto page_res = page(src, std::move(contents), std::move(hist));
  errors.insert(errors.end(), page_res.errors.begin(), page_res.errors.end());

  return ProcessResult{std::move(page_res.output), std::move(errors)};
}

auto process_template(const std::filesystem::path &src, std::string input,
                      std::unordered_set<std::filesystem::path> hist)
    -> ProcessResult {

  std::vector<ProcessError> errors;
  auto output = std::move(input);

  if (!std::regex_search(output, template_regex)) {
    return ProcessResult{std::move(output), errors};
  }

  std::vector<std::pair<std::string, std::string>> replacements;
  auto begin =
      std::sregex_iterator(output.begin(), output.end(), template_regex);
  auto end = std::sregex_iterator();

  for (auto it = begin; it != end; ++it) {
    std::smatch match = *it;

    if (is_inside_comment(output, match.position())) {
      continue;
    }

    auto found_str = match.str();
    auto template_name = match[1].str();

    auto result = get_template(src, template_name, hist);
    errors.insert(errors.end(), result.errors.begin(), result.errors.end());
    replacements.emplace_back(found_str, std::move(result.output));
  }

  for (auto it = replacements.rbegin(); it != replacements.rend(); ++it) {
    const auto &[old_val, new_val] = *it;
    auto pos = output.find(old_val);
    if (pos != std::string::npos) {
      output.replace(pos, old_val.length(), new_val);
    }
  }

  return ProcessResult{std::move(output), std::move(errors)};
}

} // namespace simple::handlers
