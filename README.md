# `rua` - A tiny utility combining many functionalities.

## `mkinfo`

Generating make information for the given platform name.

This is implemented by two related files named "src/libplatform/hs_platform.c" (a) and "scripts/platform_table" (b) respectively. Specifically as follows:
1. Read and get the corresponding records matching the given name from file a, and then extract the platform model and product long name.
2. Read and get the make information according to platform model, which is the first field of records in file b.
3. The information in file b are not containing the product name field, so we need to combine the records fetched before with each corresponding make info.
