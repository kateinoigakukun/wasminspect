extern int inc(int input);
int dec(int input) {
  return input - 1;
}
int main(void) {
  int foo = 0;
  foo++;
  foo = inc(foo);
  foo = dec(foo);
  return foo;
}
