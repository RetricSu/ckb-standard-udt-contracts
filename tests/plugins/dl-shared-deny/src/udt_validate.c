typedef unsigned long size_t;

__attribute__((visibility("default"))) int udt_validate(
    const unsigned char *script_hash, unsigned char op_type,
    unsigned char ext_index, const unsigned char *ext_data_ptr,
    unsigned long ext_data_len, unsigned char mint_authority_checked) {
  (void)script_hash;
  (void)op_type;
  (void)ext_index;
  (void)ext_data_ptr;
  (void)ext_data_len;
  (void)mint_authority_checked;

  return 1;
}
