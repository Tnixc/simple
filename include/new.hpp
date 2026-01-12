#pragma once

#include "error.hpp"
#include <string>
#include <vector>

namespace simple {

auto new_project(const std::vector<std::string> &args) -> Result<void>;

}
