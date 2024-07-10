#!/bin/bash

rm -rf ./testing
mkdir testing
cd testing
mkdir src

cd src
mkdir components
mkdir data
mkdir pages
mkdir public
mkdir templates

for i in {1..1000}; do
    random_string=$(openssl rand -base64 24 | tr -dc 'A-Z')
    # touch "components/Component$random_string.html"
    # touch "data/Data$random_string.json"
    # touch "pages/page$random_string.html"
    # touch "public/public$random_string.txt"
    # touch "templates/Template$random_string.html"

    echo "<p>Component $random_string</p>" > "components/$random_string.html"
    echo "[{\"A\": \"$random_string\"}]" > "data/$random_string.json"
    echo "<p>Template - {A} - $random_string</p>" > "templates/$random_string.html"
    echo "<p>Page $random_string</p> <br> <$random_string /> <br> <-{$random_string} />" > "pages/$random_string.html"
    # echo "Public $random_string" > "public/public$random_string.txt"

done
