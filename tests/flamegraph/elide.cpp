int goo(int x);

int foo(int x) {
    return goo(x - 1) + 1;
}
