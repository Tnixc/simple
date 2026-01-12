#include "handlers/pages.hpp"
#include "handlers/components.hpp"
#include "handlers/katex_assets.hpp"
#include "handlers/markdown.hpp"
#include "handlers/templates.hpp"
#include "minify.hpp"
#include <fstream>
#include <mutex>
#include <sstream>
#include <thread>
#include <vector>

namespace simple::handlers {

extern bool is_dev;
extern uint16_t ws_port;
extern std::string_view script;

auto page(const std::filesystem::path &src, std::string string,
          std::unordered_set<std::filesystem::path> hist) -> ProcessResult {

  std::vector<ProcessError> errors;

  if (string.find("</markdown>") != std::string::npos) {
    string = render_markdown(std::move(string));
  }

  auto wrapping_result =
      process_component(src, std::move(string), ComponentTypes::Wrapping, hist);
  string = std::move(wrapping_result.output);
  errors.insert(errors.end(), wrapping_result.errors.begin(),
                wrapping_result.errors.end());

  auto self_closing_result = process_component(
      src, std::move(string), ComponentTypes::SelfClosing, hist);
  string = std::move(self_closing_result.output);
  errors.insert(errors.end(), self_closing_result.errors.begin(),
                self_closing_result.errors.end());

  auto template_result = process_template(src, std::move(string), hist);
  string = std::move(template_result.output);
  errors.insert(errors.end(), template_result.errors.begin(),
                template_result.errors.end());

  return ProcessResult{std::move(string), std::move(errors)};
}

auto process_single_file(const std::filesystem::path &path,
                         const std::filesystem::path &dir,
                         const std::filesystem::path &src,
                         std::string_view working_dir) -> Results<void> {

  std::vector<ProcessError> errors;

  reset_katex_flag();

  std::ifstream file{path};
  if (!file) {
    return std::unexpected(std::vector{ProcessError{
        ErrorType::Io, WithItem::File, path, "Failed to read file"}});
  }

  std::stringstream buffer;
  buffer << file.rdbuf();
  auto file_content = buffer.str();

  if (file_content.empty()) {
    return std::unexpected(std::move(errors));
  }

  auto result = page(src, std::move(file_content), {});
  errors.insert(errors.end(), result.errors.begin(), result.errors.end());

  auto relative_to_src = std::filesystem::relative(path, src);
  auto relative_to_pages = std::filesystem::relative(relative_to_src, "pages");
  auto out_path = dir / working_dir / relative_to_pages;

  if (out_path.has_parent_path()) {
    std::error_code ec;
    std::filesystem::create_directories(out_path.parent_path(), ec);
    if (ec) {
      errors.push_back(ProcessError{
          ErrorType::Io, WithItem::File, out_path,
          std::format("Failed to create directory: {}", ec.message())});
      return std::unexpected(std::move(errors));
    }
  }

  auto output = std::move(result.output);

  if (was_katex_used() && !is_katex_injection_disabled()) {
    print_katex_message();

    auto katex_css = get_katex_css_tag();
    if (output.find("<head>") != std::string::npos) {
      auto head_tag = std::format("<head>\n{}", katex_css);
      size_t pos = output.find("<head>");
      output.replace(pos, 6, head_tag);
    } else {
      output = std::format("{}\n{}", katex_css, output);
    }
  }

  if (is_dev) {
    if (output.find("// * SCRIPT INCLUDED IN DEV MODE") == std::string::npos) {
      auto head_with_script = std::format("<head>{}", script);
      size_t pos = output.find("<head>");
      if (pos != std::string::npos) {
        output.replace(pos, 6, head_with_script);
      }

      size_t ws_pos = 0;
      while ((ws_pos = output.find("__SIMPLE_WS_PORT_PLACEHOLDER__", ws_pos)) !=
             std::string::npos) {
        output.replace(ws_pos, 30, std::to_string(ws_port));
        ws_pos += std::to_string(ws_port).length();
      }
    }
  }

  std::string to_write;
  if (is_dev) {
    to_write = std::move(output);
  } else {
    to_write = minify_html(output);
  }

  std::ofstream out_file{out_path, std::ios::binary};
  if (!out_file) {
    errors.push_back(ProcessError{ErrorType::Io, WithItem::File, out_path,
                                  "Failed to write file"});
    return std::unexpected(std::move(errors));
  }

  out_file << to_write;

  if (errors.empty()) {
    return {};
  }
  return std::unexpected(std::move(errors));
}

auto process_pages(const std::filesystem::path &dir,
                   const std::filesystem::path &src,
                   const std::filesystem::path &source,
                   const std::filesystem::path &pages) -> Results<void> {

  std::vector<ProcessError> errors;
  std::error_code ec;

  std::string working_dir = is_dev ? "dev" : "dist";

  std::vector<std::filesystem::path> file_tasks;
  std::vector<std::tuple<std::filesystem::path, std::filesystem::path,
                         std::filesystem::path, std::filesystem::path>>
      dir_tasks;

  for (const auto &entry : std::filesystem::directory_iterator(pages, ec)) {
    if (ec) {
      errors.push_back(ProcessError{
          ErrorType::Io, WithItem::File, pages,
          std::format("Error reading pages directory: {}", ec.message())});
      return std::unexpected(std::move(errors));
    }

    auto path = entry.path();
    if (entry.is_directory()) {
      dir_tasks.emplace_back(dir, src, source / path, path);
    } else {
      file_tasks.push_back(path);
    }
  }

  for (const auto &[d, s, src_path, p] : dir_tasks) {
    auto result = process_pages(d, s, src_path, p);
    if (!result) {
      errors.insert(errors.end(), result.error().begin(), result.error().end());
    }
  }

  std::mutex errors_mutex;
  std::vector<std::thread> threads;
  auto thread_count = std::min(std::thread::hardware_concurrency(),
                               static_cast<unsigned int>(file_tasks.size()));

  if (thread_count == 0)
    thread_count = 1;

  size_t chunk_size = (file_tasks.size() + thread_count - 1) / thread_count;

  for (size_t i = 0; i < thread_count; ++i) {
    threads.emplace_back([&, i, chunk_size]() {
      size_t start = i * chunk_size;
      size_t end = std::min(start + chunk_size, file_tasks.size());

      for (size_t j = start; j < end; ++j) {
        auto result = process_single_file(file_tasks[j], dir, src, working_dir);
        if (!result) {
          std::lock_guard lock{errors_mutex};
          errors.insert(errors.end(), result.error().begin(),
                        result.error().end());
        }
      }
    });
  }

  for (auto &thread : threads) {
    thread.join();
  }

  if (errors.empty()) {
    return {};
  }
  return std::unexpected(std::move(errors));
}

} // namespace simple::handlers
