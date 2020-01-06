### :warning: WIP :warning:
# wasminspect
An interactive debugger for WebAssembly 

```sh
$ cat foo.wat
```
```wasm
(module
  (func (export "foo") (result i32)
    (i32.const 123)
  )
)
```
```sh
$ wat2wasm foo.wat -o foo.wasm
$ wasminspect foo.wasm
(wasminspect) run foo
[I32(123)]
```
