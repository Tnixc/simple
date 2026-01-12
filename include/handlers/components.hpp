#pragma once

#include "error.hpp"
#include "utils.hpp"
#include <filesystem>
#include <string>
#include <unordered_set>
#include <vector>

namespace simple::handlers {

enum class ComponentTypes {
  SelfClosing,
  Wrapping,
};

auto get_component_self(
    const std::filesystem::path &src, std::string_view component,
    std::vector<std::pair<std::string_view, std::string_view>> targets,
    std::unordered_set<std::filesystem::path> hist) -> ProcessResult;

auto get_component_slot(
    const std::filesystem::path &src, std::string_view component,
    std::vector<std::pair<std::string_view, std::string_view>> targets,
    std::optional<std::string> slot_content,
    std::unordered_set<std::filesystem::path> hist) -> ProcessResult;

auto process_component(const std::filesystem::path &src, std::string input,
                       ComponentTypes component_type,
                       std::unordered_set<std::filesystem::path> hist)
    -> ProcessResult;

} // namespace simple::handlers
