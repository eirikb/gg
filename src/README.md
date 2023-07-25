Stages?  
gg.cmd is split into different stages.

The gg.cmd-file itself is built up like this:

1. stage1 batch script
2. stage 1 shell script
3. gzipped content
    1. stage 3 for each arch/os/variant

Roughly:

* **Stage 1**: Shell- / Batch-script for running stage2, or extracting gg.cmd (most of the file is gzip), and then
  execute stage 2.
* **Stage 2**: Shell- / PowerShell-script to execute stage 4, or use stage 3 to download stage 3, or use the OS if
  possible.
* **Stage 3**: Binary for each arch/os/variant for downloading stage 4. For example one for Linux x64 glibc, and another
  for Linux x64 musl. Building a static version with musl would be too big.
  Cosmopolitan does not (at writing time) support ARM.
* **Stage 4**: rust-based CLI. Does the actual logic (download, extract, execute). Hosted externally. One for each
  OS/arch.

The url _ggcmd.z13.web.core.windows.net_ littered around is storage for gg.eirikb.no.  
The only reason I use the direct URL instead of gg.eirikb.no is because then it won't go
through the CDN, which is, ironically, more expensive for me at the moment.  
This host will only be part of specific versions of gg.cmd, and future versions can use gg.eirikb.no insetad just fine.


