#include <bits/stdc++.h>

int main(int argc, char* argv[]) {
    if (argc != 2) {
        std::cout << "Usage: " << argv[0] << " <N>";
    }

    size_t size;
    std::stringstream{argv[1]} >> size;
    std::string str(size, 'a');

    std::regex regex{"^[a,b]*$"}; // string consists of only a and b
    
    std::cout << std::regex_match(str, regex) << '\n';
}