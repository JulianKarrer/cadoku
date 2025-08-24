
build-web:
    # remove current docs 
    rm -rf docs

    # build new bundle
    dx bundle --out-dir docs

    # generated folder to {base_url} instead of {base_url}/public
    mv docs/public/* docs

    # update the custom service worker to include all assets with
    # auto-generated paths and make a version string with current date and time
    echo "const version = \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\";" > public/cached_asset_list.js && echo "const offlineFundamentals = [" >> public/cached_asset_list.js && find docs/assets -type f | sed 's|^|    \"cadoku/|; s|$|\",|' >> public/cached_asset_list.js && sed -i '$ s/,$//' public/cached_asset_list.js && echo "];" >> public/cached_asset_list.js

    # move screenshots and custom html/js/favicon content in "public"
    cp -R screenshots docs/screenshots
    cp -R public/* docs
    # clean up the moved folder
    rm -r docs/public 

    # copy index.html to 404 so that wrong adresses redirect to the main site
    cp docs/index.html docs/404.html
