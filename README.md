# shimoheihei
a tool for messing around with CatSystem2 games

made to teach myself rust, following the *extensive* [Cs2 wiki](https://github.com/trigger-segfault/TriggersTools.CatSystem2/wiki) made by trigger-segfault 

still very much wip

[trello board](https://trello.com/b/yUxcQDFd/shimoheihei) for feature tracking

currently cli only. sorry simpletons !


## download
[latest release](https://github.com/danatationn/shimoheihei/releases/latest)

git builds (that may or not be broken) are in `Actions`
> [!NOTE]
> you need an account to download action arfifacts (files)


## usage
for now, there's only three commands: `unpack`, `list` and `info`

### unpack
unpack acts as an extraction tool and a decompilation tool. running `unpack image.int` will extract all files from the .int file, and convert them from .hg3 to .png, or whatever file format you want by passing `-c [IMAGE FORMAT]`/`--convert-hgx-to [IMAGE_FORMAT]`.

if you DON'T want shimoheihei to decompile/convert files and only to extract them, use `-r`/`--no-convert`

you can also skip files that already exist by using `-o`/`--no-overwrite`

just check `--help` i shouldn't be telling you all this

### list
for now, list does only one thing: gives you a big list of files. it has `less` controls so `q` quits and all the other binds

it also shows depth for .zt files

i plan for this to let you unpack files from within the list view and maybe view info and stuff

### info
shows very basic info about a given format . not much to comment


## compiling and stuff
it's just a regular rust crate, sooo 
``` sh
git clone https://github.com/danatationn/shimoheihei
cd shimoheihei
cargo fetch
```
if you want to compile it, `cargo build`

if you want to test it out, `cargo run info [FILE_PATH]`

<br>

#### Cardboard Creature "Type O. As in 'Oh! What a wonderful blood type.'"

## license
[GPL-3](LICENSE.txt)
