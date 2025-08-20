
build-web:
    rm -rf docs
    dx bundle --out-dir docs
    mv docs/public/* docs
    cp -R screenshots docs/screenshots
    cp -R public/* docs
