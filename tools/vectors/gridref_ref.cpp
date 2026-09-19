// Reference encodings from GeographicLib's GARS and Georef classes.
// stdin lines: "g lat lon prec" | "G code" | "r lat lon prec" | "R code"
#include <GeographicLib/GARS.hpp>
#include <GeographicLib/Georef.hpp>
#include <iostream>
#include <iomanip>
#include <sstream>
#include <string>
using namespace GeographicLib;
int main() {
  std::string line;
  std::cout << std::setprecision(17);
  while (std::getline(std::cin, line)) {
    std::istringstream in(line);
    std::string op; in >> op;
    try {
      if (op == "g" || op == "r") {
        double lat, lon; int prec; in >> lat >> lon >> prec; std::string s;
        if (op == "g") GARS::Forward(lat, lon, prec, s); else Georef::Forward(lat, lon, prec, s);
        std::cout << s << "\n";
      } else {
        std::string code; in >> code; double lat, lon; int prec;
        double clat, clon; int cprec;
        if (op == "G") { GARS::Reverse(code, lat, lon, prec, false); GARS::Reverse(code, clat, clon, cprec, true); }
        else { Georef::Reverse(code, lat, lon, prec, false); Georef::Reverse(code, clat, clon, cprec, true); }
        std::cout << lat << " " << lon << " " << clat << " " << clon << " " << prec << "\n";
      }
    } catch (const std::exception& e) { std::cout << "ERR " << e.what() << "\n"; }
  }
}
