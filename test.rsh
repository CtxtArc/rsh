#!/bin/env/rsh

for DIR in src tests target do 
    if ls $DIR 2> /dev/null then 
        echo "Found $DIR." 
    else 
        echo "Creating ${DIR:-fallback}..." 
        mkdir $DIR && echo "Success! Code: $?" || echo "Failed! Code: $?" 
    fi 
done
