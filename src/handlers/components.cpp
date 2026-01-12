#include "handlers/components.hpp"
#include "handlers/pages.hpp"
#include <fstream>
#include <regex>
#include <sstream>

namespace simple::handlers {

static const std::regex regex_self_closing{
    R"(<([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)(\s+[A-Za-z]+=(['"]).*?\4)*\s*\/>)"};

static const std::regex regex_wrapping{
    R"(<([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)(\s+[A-Za-z]+=(['"]).*?\4)*\s*>)"};

static const std::regex regex_slot{R"(<slot[\s\S]*?<\/slot>)"};

auto get_component_self(
    const std::filesystem::path &src, std::string_view component,
    std::vector<std::pair<std::string_view, std::string_view>> targets,
    std::unordered_set<std::filesystem::path> hist) -> ProcessResult {

  std::vector<ProcessError> errors;
  auto path = src / "components" /
              std::filesystem::path{component}.replace_filename(
                  std::filesystem::path{component}.filename().string());

  std::string component_str{component};
  size_t pos = 0;
  while ((pos = component_str.find(':', pos)) != std::string::npos) {
    component_str.replace(pos, 1, "/");
  }

  path = src / "components" / (component_str + ".component.html");

  if (!hist.insert(path).second) {
    return ProcessResult{"",
                         {ProcessError{ErrorType::Circular, WithItem::Component,
                                       path, "Circular dependency detected"}}};
  }

  std::ifstream file{path};
  if (!file) {
    return ProcessResult{"",
                         {ProcessError{ErrorType::Io, WithItem::Component, path,
                                       "Failed to read component file"}}};
  }

  std::stringstream buffer;
  buffer << file.rdbuf();
  auto st = buffer.str();

  if (st.empty()) {
    return ProcessResult{"", errors};
  }

  st = kv_replace(targets, std::move(st));
  auto result = page(src, std::move(st), std::move(hist));
  errors.insert(errors.end(), result.errors.begin(), result.errors.end());

  return ProcessResult{std::move(result.output), std::move(errors)};
}

auto get_component_slot(
    const std::filesystem::path &src, std::string_view component,
    std::vector<std::pair<std::string_view, std::string_view>> targets,
    std::optional<std::string> slot_content,
    std::unordered_set<std::filesystem::path> hist) -> ProcessResult {

  std::vector<ProcessError> errors;

  std::string component_str{component};
  size_t pos = 0;
  while ((pos = component_str.find(':', pos)) != std::string::npos) {
    component_str.replace(pos, 1, "/");
  }

  auto path = src / "components" / (component_str + ".component.html");

  if (!hist.insert(path).second) {
    return ProcessResult{"",
                         {ProcessError{ErrorType::Circular, WithItem::Component,
                                       path, "Circular dependency detected"}}};
  }

  std::ifstream file{path};
  if (!file) {
    return ProcessResult{"",
                         {ProcessError{ErrorType::Io, WithItem::Component, path,
                                       "Failed to read component file"}}};
  }

  std::stringstream buffer;
  buffer << file.rdbuf();
  auto st = buffer.str();

  if (st.empty()) {
    return ProcessResult{"", errors};
  }

  if (st.find("<slot>") == std::string::npos ||
      st.find("</slot>") == std::string::npos) {
    return ProcessResult{
        "",
        {ProcessError{
            ErrorType::Syntax, WithItem::Component, path,
            "The component does not contain a proper <slot></slot> tag."}}};
  }

  st = kv_replace(targets, std::move(st));

  if (slot_content) {
    st = std::regex_replace(st, regex_slot, *slot_content);
  }

  auto result = page(src, std::move(st), std::move(hist));
  errors.insert(errors.end(), result.errors.begin(), result.errors.end());

  return ProcessResult{std::move(result.output), std::move(errors)};
}

auto process_component(const std::filesystem::path &src, std::string input,
                       ComponentTypes component_type,
                       std::unordered_set<std::filesystem::path> hist)
    -> ProcessResult {

  const auto &regex = (component_type == ComponentTypes::SelfClosing)
                          ? regex_self_closing
                          : regex_wrapping;

  std::vector<ProcessError> errors;
  auto output = std::move(input);

  if (!std::regex_search(output, regex)) {
    return ProcessResult{std::move(output), errors};
  }

  std::vector<std::pair<std::string, std::string>> replacements;
  auto begin = std::sregex_iterator(output.begin(), output.end(), regex);
  auto end = std::sregex_iterator();

  for (auto it = begin; it != end; ++it) {
    std::smatch match = *it;

    if (is_inside_comment(output, match.position())) {
      continue;
    }

    auto found_str = match.str();
    auto name = match[1].str();

    auto targets_result = get_targets_kv(name, found_str);
    std::vector<std::pair<std::string, std::string>> targets;

    if (targets_result) {
      targets = std::move(*targets_result);
    } else {
      errors.push_back(targets_result.error());
      continue;
    }

    std::vector<std::pair<std::string_view, std::string_view>> target_views;
    for (const auto &[k, v] : targets) {
      target_views.emplace_back(k, v);
    }

    if (component_type == ComponentTypes::SelfClosing) {
      auto result = get_component_self(src, name, target_views, hist);
      errors.insert(errors.end(), result.errors.begin(), result.errors.end());
      replacements.emplace_back(found_str, std::move(result.output));
    } else {
      auto end = std::format("</{}>", name);
      auto slot_content = get_inside(output, found_str, end);
      auto result =
          get_component_slot(src, name, target_views, slot_content, hist);
      errors.insert(errors.end(), result.errors.begin(), result.errors.end());

      if (slot_content) {
        replacements.emplace_back(*slot_content, "");
      }
      replacements.emplace_back(end, "");
      replacements.emplace_back(found_str, std::move(result.output));
    }
  }

  for (const auto &[from, to] : replacements) {
    auto pos = output.find(from);
    if (pos != std::string::npos) {
      output.replace(pos, from.length(), to);
    }
  }

  return ProcessResult{std::move(output), std::move(errors)};
}

} // namespace simple::handlers
