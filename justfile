
build-web:
    # remove current docs 
    rm -rf docs

    # build new bundle
    dx bundle --out-dir docs

    # generated folder to {base_url} instead of {base_url}/public
    mv docs/public/* docs

    # update the custom service worker to include all assets with
    # auto-generated paths and make a version string 
    # with current date and time:

    # generate cache.tmp
    echo "const version = \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\";" > public/cache.tmp && \
    echo "const offlineFundamentals = [" >> public/cache.tmp && \
    find docs/assets -type f | sed 's|^docs/|  "cadoku/|' | sed 's|$|",|' >> public/cache.tmp && \
    sed -i '$ s/,$//' public/cache.tmp && \
    echo "];" >> public/cache.tmp

    # remove top of sw.js
    sed -n '/\/\/ END OF GENERATED CACHE FILES/,$p' ./public/sw.js > ./public/sw.tmp && mv ./public/sw.tmp ./public/sw.js

    # move cache.tmp to top of sw.js
    cat ./public/cache.tmp ./public/sw.js > ./public/sw.tmp && mv ./public/sw.tmp ./public/sw.js

    # remove cache.tmp
    rm public/cache.tmp


    # move screenshots and custom html/js/favicon content in "public"
    cp -R screenshots docs/screenshots
    cp -R public/* docs
    # clean up the moved folder
    rm -r docs/public 

    # copy index.html to 404 so that wrong adresses redirect to the main site
    cp docs/index.html docs/404.html
