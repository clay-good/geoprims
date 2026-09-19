;; A module with the geoprims ABI whose gp_invoke traps when the tool id
;; starts with "t" and otherwise returns how many calls this instance has
;; served, so a test can see a trap is contained and the instance replaced.
;; Assemble: wasm-as trap.wat -o trap.wasm
(module
  (memory (export "memory") 1)
  (global $calls (mut i32) (i32.const 0))
  (global $len (mut i32) (i32.const 0))
  ;; "{\"ok\":true,\"calls\":N}" is written at 512.
  (data (i32.const 512) "{\"ok\":true,\"calls\":0}")
  ;; A bump allocator that never frees: plenty for a test.
  (global $next (mut i32) (i32.const 1024))
  (func (export "gp_alloc") (param $n i32) (result i32)
    (global.get $next)
    (global.set $next (i32.add (global.get $next) (local.get $n))))
  (func (export "gp_free") (param i32 i32))
  (func (export "gp_out_len") (result i32) (global.get $len))
  (func (export "gp_version") (result i32) (global.set $len (i32.const 0)) (i32.const 512))
  (func (export "gp_invoke") (param $ip i32) (param $il i32) (param $xp i32) (param $xl i32) (result i32)
    (if (i32.eq (i32.load8_u (local.get $ip)) (i32.const 116)) (then unreachable))
    (global.set $calls (i32.add (global.get $calls) (i32.const 1)))
    ;; One digit is enough for the test: '0' + calls at offset 512 + 19.
    (i32.store8 (i32.const 531) (i32.add (i32.const 48) (global.get $calls)))
    (global.set $len (i32.const 21))
    (i32.const 512))
)
