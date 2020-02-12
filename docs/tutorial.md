## Command Structure

Most of wasminspect commands are similar to LLDB commands. The commands are all of the form:

```
<noun> <verb> [-options [option-value]] [argument [argument...]]
```

You can display help for each commands by `--help` flag.

## Getting started

Let's try to debug your WebAssembly binary!

Before debugging with wasminspect, please make sure that your WebAssembly binary has [DWARF debug information](http://dwarfstd.org/).

Popular compilers like `clang` produces DWARF when `-g` flag is given.

wasminspect just loads the given binary file, not execute it immediately.

```sh
$ wasminspect awesome.wasm
(wasminspect)
```

If you give commands playbook file with `--source` flag, wasminspect execute the commands after loading binary file automatically.

```sh
$ cat init_playbook
breakpoint set main
run
$ wasminspect awesome.wasm --source init_playbook
(wasminspect)
```

### Process your WebAssembly application

`run` command just starts the process. If there is another process, it confirms whether it starts new process or not.

```sh
(wasminspect) run
...
(wasminspect) run
There is a running process, kill it and restart?: [Y/n] Y
```

### Setting breakpoints

wasminspect stops process when called function contains symbols set by breakpoints.

```sh
(wasminspect) breakpoint set __original_main
(wasminspect) run
Hit breakpoint
```

### Display corresponding source file

wasminspect lists relevant source code from DWARF information.

```sh
(wasminspect) list
   1    int fib(int n) {
   2      switch (n) {
   3        case 0:
   4        case 1:
   5          return n;
   6        default:
   7          return fib(n - 2) + fib(n - 1);
   8      }
   9    }
   10
   11   int main(void) {
   12     int x = 4;
-> 13     x = fib(x);
   14     return x;
   15   }
```

### Controlling Your Program

After breakpoint hit, you can control your program by step-in, step-over, and step-out.

```sh
(wasminspect) thread step-in
(wasminspect) thread step-over
(wasminspect) thread step-out
(wasminspect) thread step-inst-in
(wasminspect) thread step-inst-over
```

You can resume the process by `process continue` command.

```sh
(wasminspect) process continue
```

### Examining Thread State

Once you’ve stopped, you can get thread information from wasminspect.

```sh
(wasminspect) thread info
0x197 `__original_main at /Users/katei/.ghq/github.com/kateinoigakukun/wasminspect/tests/simple-example/c-dwarf/main.c:5:0`
```

This result shows the instruction address, function name and source code location.

And you can examine call frame backtrace.
```sh
(wasminspect) thread backtrace
0: fib
1: fib
2: fib
3: __original_main
4: _start
```

## Experimental

### Dump frame variables

wasminspect can dump local frame variables and print their contents.

Now v0.1.0 only supports a few primitive types for `expression` command. But you can see the content by `memory` command if the content are in the linear memory.

```sh
(wasminspect) frame variable
conformance: const ProtocolConformanceDescriptor*
protocol: const ProtocolDescriptor*
requirements: ArrayRef<swift::TargetProtocolRequirement<swift::InProcess> >

(wasminspect) expression protocol
const ProtocolDescriptor* (0xe8fe8)

(wasminspect) memory read 0xe8fe8
0x000e8fe8: b4 c1 03 00 d4 a5 00 00 00 00 00 00 00 00 00 00 ................
0x000e8ff8: 94 2d 00 00 d4 a1 00 00 00 00 00 00 78 8f 0e 00 .-..........x...
```


## Advanced

### Examine WebAssembly machine status

If you're working on developing compiler, this is very useful to check the compiler emits correct instruction.

```sh
(wasminspect) global read 0
I32(67040)
(wasminspect) local read 3
I32(138)
(wasminspect) stack
0: I32(953712)
1: I32(204436)
(wasminspect) disassemble
   0x00000197: GlobalGet { global_index: 0 }
   0x0000019d: LocalSet { local_index: 0 }
-> 0x0000019f: I32Const { value: 16 }
   0x000001a1: LocalSet { local_index: 1 }
   0x000001a3: LocalGet { local_index: 0 }
   0x000001a5: LocalGet { local_index: 1 }
```


### Source Directory mapping for the binary built by other machine

If the binary is built in remote machine, DWARF records remote source directory path.
If you have same structure soruce directory in debugging machine, you can map the source directory.

This is similar feature to [LLDB's source-map.](https://lldb.llvm.org/use/map.html#miscellaneous)

> Remap source file pathnames for the debug session. If your source files are no longer located in the same location as when the program was built --- maybe the program was built on a different computer --- you need to tell the debugger how to find the sources at their local file path instead of the build system's file path.


```sh
(wasminspect) settings set directory.map /home/katei/swiftwasm-source /Users/katei/projects/swiftwasm-source
```

