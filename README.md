# Rua - A tiny utility combining many functionalities.

## clean

By running `rua clean`, you can easily remove all those build files related to last build.

There are two steps for the `clean` run:
1. Remove the "target" directory where nearly all object files are stored.
2. Remove unversioned entries within "bin" and "src" directory as these files
are generated automatically.

`rua clean` runs faster than `make dist_clean`, as make invokes `rm` commands
too frequently.

## mkinfo

Generating make information for the given platform name.

This is implemented by two related files named "src/libplatform/hs_platform.c" (a) and "scripts/platform_table" (b) respectively. Specifically as follows:

1. Read and get the corresponding records matching the given name from file a, and then extract the platform model and product long name.
2. Read and get the make information according to platform model, which is the first field of records in file b.
3. The information in file b are not containing the product name field, so we need to combine the records fetched before with each corresponding make info.

## compdb

Generating JSON compilation database for the given target.

This command requires two parameters which are *path* and *target*
respectively. *path* is actually the make path for one target which can be
handled automatically by make's -C option.

It's worth noting that, this command must be run after a successfully
compilation as there are so many source files generated by scripts
from XML files.

## silist

This utility is used for generating a master filelist for Source Insight.

Because we run under Linux environment but Source Insight runs over Windows,
We need give the full path to the project root to the command. For example:

```shell
rua silist 'F:/repos/MX_MAIN'
```

## showcc

Users can use this command to fetch the compile command for a specific filename. This is useful for who want to check the compilation error only.

> This functionality depends on JSON Compilation database. So you have to
> generate it using `compdb` command firstly. Besides, the output is
> specific to the product used to generate the JSON compilation database.

## init

Use this command to generate completion scripts for a specific shell. With auto-completion scripts, rua is easier to use.

```shell
eval "$(rua init bash)"
```
