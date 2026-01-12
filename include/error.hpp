#pragma once

#include <expected>
#include <filesystem>
#include <format>
#include <string>
#include <vector>

namespace simple {

enum class ErrorType {
  Io,
  Syntax,
  Circular,
  Other,
};

enum class WithItem {
  Component,
  Template,
  Data,
  File,
  None,
};

inline auto to_string(WithItem item) -> std::string {
  switch (item) {
  case WithItem::Component:
    return "component";
  case WithItem::Template:
    return "template";
  case WithItem::Data:
    return "data";
  case WithItem::File:
    return "file or directory";
  case WithItem::None:
    return "item";
  }
  return "item";
}

struct ProcessError {
  ErrorType error_type;
  WithItem item;
  std::filesystem::path path;
  std::string message;

  auto format() const -> std::string {
    auto item_str = to_string(item);
    auto path_str = path.string();
    auto msg_fmt =
        message.empty() ? "" : std::format("\033[1m{}\033[0m", message);

    switch (error_type) {
    case ErrorType::Io:
      return std::format("The {} \033[31m{}\033[0m encountered an IO error. {}",
                         item_str, path_str, msg_fmt);
    case ErrorType::Syntax:
      return std::format("The {} \033[31m{}\033[0m contains a syntax error. {}",
                         item_str, path_str, msg_fmt);
    case ErrorType::Circular:
      return std::format(
          "The {} \033[31m{}\033[0m contains a circular dependency.", item_str,
          path_str);
    case ErrorType::Other:
      return std::format("Error encountered in {} \033[31m{}\033[0m. {}",
                         item_str, path_str, msg_fmt);
    }
    return "";
  }
};

template <typename T> using Result = std::expected<T, ProcessError>;

template <typename T>
using Results = std::expected<T, std::vector<ProcessError>>;

} // namespace simple
