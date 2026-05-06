__attribute__((visibility("default"))) int udt_authorize(
    const unsigned char *script_hash, const unsigned char *args,
    unsigned long args_len) {
  (void)script_hash;
  (void)args;
  (void)args_len;
  return 1;
}
