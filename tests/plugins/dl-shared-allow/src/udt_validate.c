typedef unsigned long size_t;

static int matches_literal(const unsigned char *data, size_t len,
                           const unsigned char *expected,
                           size_t expected_len) {
  size_t index = 0;

  if (len != expected_len) {
    return 0;
  }

  for (index = 0; index < expected_len; index++) {
    if (data[index] != expected[index]) {
      return 0;
    }
  }
  return 1;
}

#define MATCHES(data, len, literal)                                            \
  matches_literal((data), (len), (const unsigned char *)(literal),             \
                  sizeof(literal) - 1)

static int validate_policy(unsigned char op_type, unsigned char ext_index,
                           const unsigned char *ext_data_ptr,
                           unsigned long ext_data_len,
                           unsigned char mint_authority_checked) {
  if (ext_data_len == 0) {
    return 0;
  }
  if (MATCHES(ext_data_ptr, ext_data_len, "require_mint_checked")) {
    return mint_authority_checked == 1 ? 0 : 1;
  }
  if (MATCHES(ext_data_ptr, ext_data_len, "require_mint_none")) {
    return mint_authority_checked == 2 ? 0 : 2;
  }
  if (MATCHES(ext_data_ptr, ext_data_len, "require_transfer")) {
    return op_type == 0 ? 0 : 3;
  }
  if (MATCHES(ext_data_ptr, ext_data_len, "require_mint")) {
    return op_type == 1 ? 0 : 4;
  }
  if (MATCHES(ext_data_ptr, ext_data_len, "require_protocol_burn")) {
    return op_type == 2 ? 0 : 5;
  }
  if (MATCHES(ext_data_ptr, ext_data_len, "require_index_0")) {
    return ext_index == 0 ? 0 : 6;
  }
  if (MATCHES(ext_data_ptr, ext_data_len, "require_index_1")) {
    return ext_index == 1 ? 0 : 7;
  }

  return 8;
}

__attribute__((visibility("default"))) int udt_validate(
    const unsigned char *script_hash, unsigned char op_type,
    unsigned char ext_index, const unsigned char *ext_data_ptr,
    unsigned long ext_data_len, unsigned char mint_authority_checked) {
  (void)script_hash;

  return validate_policy(op_type, ext_index, ext_data_ptr, ext_data_len,
                         mint_authority_checked);
}
