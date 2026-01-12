#include "dev/dev.hpp"
#include "dev/inline_script.hpp"
#include "error.hpp"
#include "handlers/pages.hpp"
#include "new.hpp"
#include "utils.hpp"
#include "version.hpp"
#include <chrono>
#include <fmt/core.h>
#include <iostream>

namespace simple {
bool is_dev = false;
}

namespace simple::handlers {
bool is_dev = false;
uint16_t ws_port = 0;
std::string_view script = simple::dev::INLINE_SCRIPT;
} // namespace simple::handlers

namespace simple::dev {
extern uint16_t ws_port;
extern bool is_dev;
} // namespace simple::dev

auto build(const std::vector<std::string> &args) -> simple::Results<void> {
  fmt::print("\033[36m\033[1mBuilding\033[0m...\n");

  auto start = std::chrono::steady_clock::now();

  if (args.size() < 3) {
    return {};
  }

  auto dir = std::filesystem::path{args[2]};
  auto src = dir / "src";

  std::string working_dir = simple::is_dev ? "dev" : "dist";
  auto dist = dir / working_dir;

  auto pages = src / "pages";
  auto public_dir = src / "public";

  if (!std::filesystem::exists(dir / working_dir)) {
    std::error_code ec;
    std::filesystem::create_directory(dir / working_dir, ec);
    if (ec) {
      return std::unexpected(std::vector{
          simple::ProcessError{simple::ErrorType::Io, simple::WithItem::File,
                               dir / working_dir, ec.message()}});
    }
  }

  auto page_result = simple::handlers::process_pages(dir, src, src, pages);
  if (!page_result) {
    return std::unexpected(page_result.error());
  }

  auto copy_result = simple::copy_into(public_dir, dist);
  if (!copy_result) {
    return std::unexpected(std::vector{copy_result.error()});
  }

  auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(
                      std::chrono::steady_clock::now() - start)
                      .count();

  fmt::print("\033[32m\033[1mDone\033[0m in {} ms.\n", duration);

  return {};
}

auto main(int argc, char *argv[]) -> int {
  std::vector<std::string> args;
  for (int i = 0; i < argc; ++i) {
    args.emplace_back(argv[i]);
  }

  if (args.size() > 1 && (args[1] == "--version" || args[1] == "-v")) {
    fmt::print("simple {} ({})\n", simple::VERSION, simple::GIT_HASH);
    return 0;
  }

  if (args.size() < 3) {
    fmt::print(stderr, "\033[1m\033[31mError\033[0m: Not enough arguments. "
                       "Usage: simple [operation] [dir]\n");
    return 1;
  }

  const auto &command = args[1];

  if (command == "dev") {
    simple::is_dev = true;
    simple::handlers::is_dev = true;
    simple::dev::is_dev = true;
    simple::dev::spawn_watcher(args);
    return 0;
  } else if (command == "build") {
    simple::is_dev = false;
    simple::handlers::is_dev = false;
    auto result = build(args);
    if (!result) {
      simple::print_vec_errs(result.error());
      return 1;
    }
  } else if (command == "new") {
    auto result = simple::new_project(args);
    if (!result) {
      fmt::print(stderr, "\033[1m\033[31mScaffold error\033[0m: {}\n",
                 result.error().format());
      return 1;
    }
  } else {
    fmt::print("Unknown operation. Operations: build, dev, new\n");
    return 1;
  }

  return 0;
}
