#include <iostream>
#include <string>

#include "s2/s2cell_id.h"
#include "s2/s2latlng.h"

int main() {
  const auto point = S2LatLng::FromDegrees(53.92627875804599, 62.1184848380413);
  const auto cell = S2CellId(point.ToPoint()).parent(10);
  const std::string token = cell.ToToken();
  if (token != "43ce61") {
    std::cerr << "S2 smoke mismatch: expected 43ce61, got " << token << '\n';
    return 1;
  }
  std::cout << token << '\n';
}
