# ffs

I am tired of all the shit to wade through when building and testing code. I hate making tools work together and getting a reliable, fast, incremental build system set up for every new project.

This is a Fast, Flexible, and Simple build system for code, ffs. Any similarity to internet slang is purely coincidental.

## Goals

```sh
git clone <my-project>
cd <my-project>
./ffs test # All tests run and pass
```

* Users don't need to install extra tools. Just clone and run.
* Integration with new languages, frameworks, or custom scripts is simple and works with existing plugins or other custom scripts.
* Builds produce byte-for-byte identical output if the underlying tools do.

## Model

ffs commands:

* build - Construct all artifacts.
* test - Run tests.
* deploy - Run all "deploy" actions.
