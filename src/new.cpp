#include "new.hpp"
#include <filesystem>
#include <fstream>
#include <iostream>

namespace simple {

constexpr std::string_view INDEX = R"(
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Simple Demo</title>
</head>
<body>
  <h1>Welcome to simple. Take a look at <a href="https://github.com/Tnixc/simple">the github</a> to get started</h1>
</body>
</html>
)";

auto new_project(const std::vector<std::string> &args) -> Result<void> {
  if (args.size() < 3) {
    return std::unexpected(
        ProcessError{ErrorType::Other,
                     WithItem::None,
                     {},
                     "Not enough arguments for 'new' command"});
  }

  auto path = std::filesystem::path{args[2]};
  std::error_code ec;

  std::filesystem::create_directory(path, ec);
  if (ec) {
    return std::unexpected(
        ProcessError{ErrorType::Io, WithItem::File, path, ec.message()});
  }

  auto src = path / "src";
  std::filesystem::create_directory(src, ec);
  if (ec) {
    return std::unexpected(
        ProcessError{ErrorType::Io, WithItem::File, src, ec.message()});
  }

  auto dirs = {src / "components", src / "templates", src / "data",
               src / "public", src / "pages"};

  for (const auto &dir : dirs) {
    std::filesystem::create_directory(dir, ec);
    if (ec) {
      return std::unexpected(
          ProcessError{ErrorType::Io, WithItem::File, dir, ec.message()});
    }
  }

  auto index_path = src / "pages" / "index.html";
  std::ofstream index_file{index_path};
  if (!index_file) {
    return std::unexpected(ProcessError{ErrorType::Io, WithItem::File,
                                        index_path,
                                        "Failed to create index.html"});
  }

  index_file << INDEX;

  std::cout << std::format("Done. run `simple build {}` to get started\n",
                           args[2]);

  return {};
}

} // namespace simple
