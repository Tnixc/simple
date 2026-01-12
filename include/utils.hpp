#pragma once

#include "error.hpp"
#include <filesystem>
#include <string>
#include <string_view>
#include <vector>

namespace simple {

struct ProcessResult {
  std::string output;
  std::vector<ProcessError> errors;
};

auto is_inside_comment(const std::string &text, size_t pos) -> bool;

auto get_targets_kv(std::string_view name, std::string_view found)
    -> Result<std::vector<std::pair<std::string, std::string>>>;

auto kv_replace(
    const std::vector<std::pair<std::string_view, std::string_view>> &kv,
    std::string from) -> std::string;

auto get_inside(const std::string &input, std::string_view from,
                std::string_view to) -> std::optional<std::string>;

auto copy_into(const std::filesystem::path &public_dir,
               const std::filesystem::path &dist) -> Result<void>;

auto unindent(std::string_view input) -> std::string;

auto print_vec_errs(const std::vector<ProcessError> &errors) -> void;

auto format_errs(const std::vector<ProcessError> &errors) -> std::string;

auto walk_dir(const std::filesystem::path &dir)
    -> Result<std::vector<std::filesystem::path>>;

auto find_next_available_port(uint16_t start_port) -> uint16_t;

auto is_port_available(uint16_t port) -> bool;

} // namespace simple
