# Usage

Change the three strings in src/main.rs:

  - SAVE_DIR - directory where the images will be stored
  - SPOTLIGHT_DIR - location of Windows spotlight in your PC
  - SAVED_HASH - filename for the cached hashes of previously saved images

Build with cargo
```sh
cargo build --release
```

To make it run at boot, place the resulting executable in *C:\Users\YourUsername\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup*