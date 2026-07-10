(module
  (memory (export "memory") 1 2)
  (data (i32.const 64) "[{\"symbol\":\"@107\",\"label\":\"plugin:gap\",\"detail\":\"read-only wasm annotation\"}]")
  (func (export "annotate_row") (param i32) (param i32) (result i32)
    i32.const 0)
  (func (export "hls_output_ptr") (result i32)
    i32.const 64)
  (func (export "hls_output_len") (result i32)
    i32.const 77))
