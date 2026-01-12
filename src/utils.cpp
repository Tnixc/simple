#include "utils.hpp"
#include <algorithm>
#include <fmt/core.h>
#include <netinet/in.h>
#include <ranges>
#include <regex>
#include <sys/socket.h>
#include <unistd.h>

namespace simple {

static const std::regex kv_regex{R"((\w+)=(['"])(.*?)\2)"};

auto is_inside_comment(const std::string &text, size_t pos) -> bool {
  size_t comment_start = text.rfind("<!--", pos);
  if (comment_start == std::string::npos)
    return false;

  size_t comment_end = text.find("-->", comment_start);
  if (comment_end == std::string::npos)
    return false;

  return pos >= comment_start && pos <= comment_end + 3;
}

auto get_targets_kv(std::string_view name, std::string_view found)
    -> Result<std::vector<std::pair<std::string, std::string>>> {

  std::vector<std::pair<std::string, std::string>> targets;
  auto start_tag = std::format("<{}", name);

  auto trimmed = found;
  if (trimmed.starts_with(start_tag)) {
    trimmed.remove_prefix(start_tag.size());
  }

  while (trimmed.ends_with('>') || trimmed.ends_with("/>")) {
    if (trimmed.ends_with("/>")) {
      trimmed.remove_suffix(2);
    } else {
      trimmed.remove_suffix(1);
    }
  }

  std::string trimmed_str{trimmed};
  auto begin =
      std::sregex_iterator(trimmed_str.begin(), trimmed_str.end(), kv_regex);
  auto end = std::sregex_iterator();

  for (auto it = begin; it != end; ++it) {
    std::smatch match = *it;
    if (match.size() >= 4) {
      targets.emplace_back(match[1].str(), match[3].str());
    }
  }

  return targets;
}

auto kv_replace(
    const std::vector<std::pair<std::string_view, std::string_view>> &kv,
    std::string from) -> std::string {
  if (kv.empty()) {
    return from;
  }

  auto result = std::move(from);
  for (const auto &[k, v] : kv) {
    auto key = std::format("${{{}}}", k);
    size_t pos = 0;
    while ((pos = result.find(key, pos)) != std::string::npos) {
      result.replace(pos, key.length(), v);
      pos += v.length();
    }
  }
  return result;
}

auto get_inside(const std::string &input, std::string_view from,
                std::string_view to) -> std::optional<std::string> {

  auto start_index = input.find(from);
  if (start_index == std::string::npos)
    return std::nullopt;

  auto start_pos = start_index + from.length();
  auto end_index = input.find(to, start_pos);
  if (end_index == std::string::npos)
    return std::nullopt;

  if (start_pos >= end_index) {
    return std::nullopt;
  }

  return input.substr(start_pos, end_index - start_pos);
}

auto copy_into(const std::filesystem::path &public_dir,
               const std::filesystem::path &dist) -> Result<void> {

  if (!std::filesystem::exists(dist)) {
    std::error_code ec;
    std::filesystem::create_directories(dist, ec);
    if (ec) {
      return std::unexpected(
          ProcessError{ErrorType::Io, WithItem::File, dist, ec.message()});
    }
  }

  std::error_code ec;
  for (const auto &entry :
       std::filesystem::recursive_directory_iterator(public_dir, ec)) {
    if (ec) {
      return std::unexpected(ProcessError{ErrorType::Io, WithItem::File,
                                          public_dir, ec.message()});
    }

    auto relative = std::filesystem::relative(entry.path(), public_dir, ec);
    if (ec) {
      return std::unexpected(ProcessError{
          ErrorType::Io, WithItem::File, entry.path(),
          std::format("Failed to strip prefix: {}", ec.message())});
    }

    auto dest_path = dist / relative;

    if (entry.is_directory()) {
      std::filesystem::create_directories(dest_path, ec);
      if (ec) {
        return std::unexpected(ProcessError{ErrorType::Io, WithItem::File,
                                            dest_path, ec.message()});
      }
    } else {
      if (dest_path.has_parent_path()) {
        std::filesystem::create_directories(dest_path.parent_path(), ec);
        if (ec) {
          return std::unexpected(ProcessError{ErrorType::Io, WithItem::File,
                                              dest_path, ec.message()});
        }
      }
      std::filesystem::copy(entry.path(), dest_path,
                            std::filesystem::copy_options::overwrite_existing,
                            ec);
      if (ec) {
        return std::unexpected(ProcessError{ErrorType::Io, WithItem::File,
                                            dest_path, ec.message()});
      }
    }
  }

  return {};
}

auto unindent(std::string_view input) -> std::string {
  std::vector<std::string_view> lines;
  size_t start = 0;

  for (size_t i = 0; i <= input.size(); ++i) {
    if (i == input.size() || input[i] == '\n') {
      lines.push_back(input.substr(start, i - start));
      start = i + 1;
    }
  }

  if (lines.empty()) {
    return std::string{};
  }

  size_t min_indent = std::numeric_limits<size_t>::max();
  for (const auto &line : lines) {
    if (line.find_first_not_of(" \t") == std::string_view::npos)
      continue;

    size_t indent = 0;
    while (indent < line.size() &&
           (line[indent] == ' ' || line[indent] == '\t')) {
      ++indent;
    }
    min_indent = std::min(min_indent, indent);
  }

  if (min_indent == std::numeric_limits<size_t>::max()) {
    min_indent = 0;
  }

  std::string result;
  result.reserve(input.size());

  for (size_t i = 0; i < lines.size(); ++i) {
    if (i > 0)
      result += '\n';

    const auto &line = lines[i];
    if (line.size() > min_indent &&
        line.find_first_not_of(" \t") != std::string_view::npos) {
      result += line.substr(min_indent);
    } else {
      auto trimmed_start = line.find_first_not_of(" \t");
      if (trimmed_start != std::string_view::npos) {
        result += line.substr(trimmed_start);
      }
    }
  }

  return result;
}

auto print_vec_errs(const std::vector<ProcessError> &errors) -> void {
  for (size_t i = 0; i < errors.size(); ++i) {
    fmt::print(stderr, "\033[1m\033[31mBuild error {}\033[0m: {}\n", i + 1,
               errors[i].format());
  }
}

auto format_errs(const std::vector<ProcessError> &errors) -> std::string {
  std::string msg;
  msg.reserve(errors.size() * 100);

  for (size_t i = 0; i < errors.size(); ++i) {
    msg += std::format("<p>\033[1m\033[31mBuild error {}\033[0m: {}\n</p>",
                       i + 1, errors[i].format());
  }

  return msg;
}

auto walk_dir(const std::filesystem::path &dir)
    -> Result<std::vector<std::filesystem::path>> {
  std::vector<std::filesystem::path> files;
  std::error_code ec;

  for (const auto &entry :
       std::filesystem::recursive_directory_iterator(dir, ec)) {
    if (ec) {
      return std::unexpected(
          ProcessError{ErrorType::Io, WithItem::File, dir, ec.message()});
    }

    if (entry.is_regular_file()) {
      files.push_back(entry.path());
    }
  }

  return files;
}

auto is_port_available(uint16_t port) -> bool {
  int sockfd = socket(AF_INET, SOCK_STREAM, 0);
  if (sockfd < 0)
    return false;

  sockaddr_in addr{};
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = INADDR_ANY;
  addr.sin_port = htons(port);

  bool available =
      bind(sockfd, reinterpret_cast<sockaddr *>(&addr), sizeof(addr)) == 0;
  close(sockfd);

  return available;
}

auto find_next_available_port(uint16_t start_port) -> uint16_t {
  for (uint16_t port = start_port; port < 65535; ++port) {
    if (is_port_available(port)) {
      return port;
    }
  }
  return start_port;
}

} // namespace simple
