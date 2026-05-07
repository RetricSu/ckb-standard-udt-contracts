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

static int policy_shape_is_valid(unsigned char op_type, unsigned char ext_index,
                                 const unsigned char *ext_data_ptr,
                                 unsigned long ext_data_len,
                                 unsigned char mint_authority_checked) {
  if (op_type > 2 || ext_index > 15 || mint_authority_checked > 2) {
    return 0;
  }
  if (ext_data_len == 0) {
    return 1;
  }
  if (MATCHES(ext_data_ptr, ext_data_len, "require_mint_checked") ||
      MATCHES(ext_data_ptr, ext_data_len, "require_mint_none") ||
      MATCHES(ext_data_ptr, ext_data_len, "require_transfer") ||
      MATCHES(ext_data_ptr, ext_data_len, "require_mint") ||
      MATCHES(ext_data_ptr, ext_data_len, "require_protocol_burn") ||
      MATCHES(ext_data_ptr, ext_data_len, "require_index_0") ||
      MATCHES(ext_data_ptr, ext_data_len, "require_index_1")) {
    return 1;
  }

  return 0;
}

__attribute__((visibility("default"))) int udt_validate(
    const unsigned char *script_hash, unsigned char op_type,
    unsigned char ext_index, const unsigned char *ext_data_ptr,
    unsigned long ext_data_len, unsigned char mint_authority_checked) {
  (void)script_hash;
  if (!policy_shape_is_valid(op_type, ext_index, ext_data_ptr, ext_data_len,
                             mint_authority_checked)) {
    return 2;
  }

  return 1;
}
