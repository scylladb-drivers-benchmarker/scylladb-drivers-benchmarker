#include <bits/stdc++.h>

int main(int argc, char* argv[]) {
    if (argc != 2) {
        std::cout << "Usage: " << argv[0] << " <N>";
        return -1;
    }

    size_t size;
    std::stringstream{argv[1]} >> size;
    std::string str(size, 'a');

    for (auto c : str) {
        if (c != 'a') {
            return 1;
        }
    }
}