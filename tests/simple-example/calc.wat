(module
  (func $add (export "add") (param i32) (param i32) (result i32)
    local.get 0
    local.get 1
    i32.add)
  (func $mul (export "mul") (param $n i32) (param $m i32) (result i32)  (local $i i32) (local $sum i32)
    (block $exit
      (loop $loop
        (br_if $exit (i32.lt_s (get_local $n) (get_local $i)))
        (set_local $sum (i32.add (get_local $sum) (get_local $n)))
        (set_local $i (i32.add (get_local $i) (i32.const 1)))
        (br $loop)))
    (return (get_local $sum)))
)
