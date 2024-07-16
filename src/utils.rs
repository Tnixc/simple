use crate::error::rewrite_error;
use crate::error::ErrorType;
use crate::error::PageHandleError;
use crate::error::WithItem;
use fancy_regex::Regex;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::str;
use std::{io::Error, path::PathBuf};
use ErrorType::Io;
use ErrorType::NotFound;
use ErrorType::Utf8;
use WithItem::Component;
use WithItem::Data;
use WithItem::File;
use WithItem::Template;

const COMPONENT_PATTERN_OPEN: &str =
    r#"(?<!<!--)<([A-Z][A-Za-z_]*(\/[A-Z][A-Za-z_]*)*)(\s+[a-z]+=(["'])[^"']*\4)*\s*>(?!.*?-->)"#;

// const _COMPONENT_PATTERN_: &str =
//     r"(?<!<!--)<([A-Z][A-Za-z_]*(\/[A-Z][A-Za-z_]*)*)\s*\/>(?!.*?-->)";

const TEMPLATE_PATTERN: &str =
    r"(?<!<!--)<-\{([A-Z][A-Za-z_]*(\/[A-Z][A-Za-z_]*)*)\}\s*\/>(?!.*?-->)";
// Thank you ChatGPT, couldn't have done this Regex-ing without you.

const SCRIPT: &str = r#"
<script>
/*
 Live.js - One script closer to Designing in the Browser
 Written for Handcraft.com by Martin Kool (@mrtnkl).

 Version 4.
 Recent change: Made stylesheet and mimetype checks case insensitive.

 http://livejs.com
 http://livejs.com/license (MIT)
 @livejs

 Include live.js#css to monitor css changes only.
 Include live.js#js to monitor js changes only.
 Include live.js#html to monitor html changes only.
 Mix and match to monitor a preferred combination such as live.js#html,css

 By default, just include live.js to monitor all css, js and html changes.

 Live.js can also be loaded as a bookmarklet. It is best to only use it for CSS then,
 as a page reload due to a change in html or css would not re-include the bookmarklet.
 To monitor CSS and be notified that it has loaded, include it as: live.js#css,notify
*/
(function () {

 var headers = { "Etag": 1, "Last-Modified": 1, "Content-Length": 1, "Content-Type": 1 },
     resources = {},
     pendingRequests = {},
     currentLinkElements = {},
     oldLinkElements = {},
     interval = 100,
     loaded = false,
     active = { "html": 1, "css": 1, "js": 1 };

 var Live = {

   // performs a cycle per interval
   heartbeat: function () {
     if (document.body) {
       // make sure all resources are loaded on first activation
       if (!loaded) Live.loadresources();
       Live.checkForChanges();
     }
     setTimeout(Live.heartbeat, interval);
   },

   // loads all local css and js resources upon first activation
   loadresources: function () {

     // helper method to assert if a given url is local
     function isLocal(url) {
       var loc = document.location,
           reg = new RegExp("^\\.|^\/(?!\/)|^[\\w]((?!://).)*$|" + loc.protocol + "//" + loc.host);
       return url.match(reg);
     }

     // gather all resources
     var scripts = document.getElementsByTagName("script"),
         links = document.getElementsByTagName("link"),
         uris = [];

     // track local js urls
     for (var i = 0; i < scripts.length; i++) {
       var script = scripts[i], src = script.getAttribute("src");
       if (src && isLocal(src))
         uris.push(src);
       if (src && src.match(/\blive.js#/)) {
         for (var type in active)
           active[type] = src.match("[#,|]" + type) != null
         if (src.match("notify"))
           alert("Live.js is loaded.");
       }
     }
     if (!active.js) uris = [];
     if (active.html) uris.push(document.location.href);

     // track local css urls
     for (var i = 0; i < links.length && active.css; i++) {
       var link = links[i], rel = link.getAttribute("rel"), href = link.getAttribute("href", 2);
       if (href && rel && rel.match(new RegExp("stylesheet", "i")) && isLocal(href)) {
         uris.push(href);
         currentLinkElements[href] = link;
       }
     }

     // initialize the resources info
     for (var i = 0; i < uris.length; i++) {
       var url = uris[i];
       Live.getHead(url, function (url, info) {
         resources[url] = info;
       });
     }

     // add rule for morphing between old and new css files
     var head = document.getElementsByTagName("head")[0],
         style = document.createElement("style"),
         rule = "transition: all .3s ease-out;"
     css = [".livejs-loading * { ", rule, " -webkit-", rule, "-moz-", rule, "-o-", rule, "}"].join('');
     style.setAttribute("type", "text/css");
     head.appendChild(style);
     style.styleSheet ? style.styleSheet.cssText = css : style.appendChild(document.createTextNode(css));

     // yep
     loaded = true;
   },

   // check all tracking resources for changes
   checkForChanges: function () {
     for (var url in resources) {
       if (pendingRequests[url])
         continue;

       Live.getHead(url, function (url, newInfo) {
         var oldInfo = resources[url],
             hasChanged = false;
         resources[url] = newInfo;
         for (var header in oldInfo) {
           // do verification based on the header type
           var oldValue = oldInfo[header],
               newValue = newInfo[header],
               contentType = newInfo["Content-Type"];
           switch (header.toLowerCase()) {
             case "etag":
               if (!newValue) break;
               // fall through to default
             default:
               hasChanged = oldValue != newValue;
               break;
           }
           // if changed, act
           if (hasChanged) {
             Live.refreshResource(url, contentType);
             break;
           }
         }
       });
     }
   },

   // act upon a changed url of certain content type
   refreshResource: function (url, type) {
     switch (type.toLowerCase()) {
       // css files can be reloaded dynamically by replacing the link element
       case "text/css":
         var link = currentLinkElements[url],
             html = document.body.parentNode,
             head = link.parentNode,
             next = link.nextSibling,
             newLink = document.createElement("link");

         html.className = html.className.replace(/\s*livejs\-loading/gi, '') + ' livejs-loading';
         newLink.setAttribute("type", "text/css");
         newLink.setAttribute("rel", "stylesheet");
         newLink.setAttribute("href", url + "?now=" + new Date() * 1);
         next ? head.insertBefore(newLink, next) : head.appendChild(newLink);
         currentLinkElements[url] = newLink;
         oldLinkElements[url] = link;

         // schedule removal of the old link
         Live.removeoldLinkElements();
         break;

       // check if an html resource is our current url, then reload
       case "text/html":
         if (url != document.location.href)
           return;

         // local javascript changes cause a reload as well
       case "text/javascript":
       case "application/javascript":
       case "application/x-javascript":
         document.location.reload();
     }
   },

   // removes the old stylesheet rules only once the new one has finished loading
   removeoldLinkElements: function () {
     var pending = 0;
     for (var url in oldLinkElements) {
       // if this sheet has any cssRules, delete the old link
       try {
         var link = currentLinkElements[url],
             oldLink = oldLinkElements[url],
             html = document.body.parentNode,
             sheet = link.sheet || link.styleSheet,
             rules = sheet.rules || sheet.cssRules;
         if (rules.length >= 0) {
           oldLink.parentNode.removeChild(oldLink);
           delete oldLinkElements[url];
           setTimeout(function () {
             html.className = html.className.replace(/\s*livejs\-loading/gi, '');
           }, 100);
         }
       } catch (e) {
         pending++;
       }
       if (pending) setTimeout(Live.removeoldLinkElements, 50);
     }
   },

   // performs a HEAD request and passes the header info to the given callback
   getHead: function (url, callback) {
     pendingRequests[url] = true;
     var xhr = window.XMLHttpRequest ? new XMLHttpRequest() : new ActiveXObject("Microsoft.XmlHttp");
     xhr.open("HEAD", url, true);
     xhr.onreadystatechange = function () {
       delete pendingRequests[url];
       if (xhr.readyState == 4 && xhr.status != 304) {
         xhr.getAllResponseHeaders();
         var info = {};
         for (var h in headers) {
           var value = xhr.getResponseHeader(h);
           // adjust the simple Etag variant to match on its significant part
           if (h.toLowerCase() == "etag" && value) value = value.replace(/^W\//, '');
           if (h.toLowerCase() == "content-type" && value) value = value.replace(/^(.*?);.*?$/i, "$1");
           info[h] = value;
         }
         callback(url, info);
       }
     }
     xhr.send();
   }
 };

 // start listening
 if (document.location.protocol != "file:") {
   if (!window.liveJsLoaded)
     Live.heartbeat();

   window.liveJsLoaded = true;
 }
 else if (window.console)
   console.log("Live.js doesn't support the file protocol. It needs http.");
})();
</script>
"#;

pub fn sub_component(src: &PathBuf, component: &str) -> Result<String, PageHandleError> {
    let path = src
        .join("components")
        .join(component)
        .with_extension("component.html");

    let contents = rewrite_error(fs::read(path.clone()), Component, NotFound, &path)?;

    return page(src, contents, false);
}

pub fn sub_template(src: &PathBuf, name: &str) -> Result<String, PageHandleError> {
    let template_path = src
        .join("templates")
        .join(name)
        .with_extension("template.html");
    let data_path = src.join("data").join(name).with_extension("data.json");

    let template_content_utf =
        rewrite_error(fs::read(&template_path), Template, NotFound, &template_path)?;

    let template = rewrite_error(
        String::from_utf8(template_content_utf),
        Template,
        Utf8,
        &template_path,
    )?;

    let data_content_utf8 = rewrite_error(fs::read(&data_path), Data, NotFound, &data_path)?;
    let data_str = str::from_utf8(&data_content_utf8).unwrap();
    let v: Value = serde_json::from_str(data_str).expect("JSON decode error");
    let items = v.as_array().expect("JSON wasn't an array");

    let mut contents = String::new();
    for object in items {
        let mut this = template.clone();
        for (key, value) in object.as_object().expect("Invalid object in JSON") {
            let key = format!("{{{}}}", key);
            this = this.replace(
                key.as_str(),
                value
                    .as_str()
                    .expect("JSON object value couldn't be decoded to string"),
            );
        }
        contents.push_str(&this);
    }

    return page(src, contents.into_bytes(), false);
}

fn page(src: &PathBuf, contents: Vec<u8>, dev: bool) -> Result<String, PageHandleError> {
    let mut string = rewrite_error(String::from_utf8(contents), File, Io, src)?;

    let re_component =
        Regex::new(COMPONENT_PATTERN_OPEN).expect("Regex failed to parse. This shouldn't happen.");
    for f in re_component.find_iter(&string.clone()) {
        if f.is_ok() {
            let found = f.unwrap();
            let trim = found
                .as_str()
                .trim()
                .trim_start_matches("<")
                .trim_end_matches(">")
                .trim();
            string = string.replace(
                found.as_str(),
                &sub_component(src, trim.split_once(" ").map(|(f, _)| f).unwrap_or(trim))?,
            );
            println!("Using: {:?}", found.as_str())
        }
    }

    let re_template =
        Regex::new(TEMPLATE_PATTERN).expect("Regex failed to parse. This shouldn't happen.");
    for f in re_template.find_iter(&string.clone()) {
        if f.is_ok() {
            let found = f.unwrap();
            string = string.replace(
                found.as_str(),
                &sub_template(
                    src,
                    found
                        .as_str()
                        .trim()
                        .trim_start_matches("<-{")
                        .trim_end_matches("/>")
                        .trim()
                        .trim_end_matches("}"),
                )?,
            );
            println!("Using: {:?}", found.as_str())
        }
    }

    if dev {
        string = string.replace("<head>", ("<head>".to_owned() + SCRIPT).as_str());
    }
    return Ok(string);
}

pub fn process_pages(
    dir: &PathBuf,
    src: &PathBuf,
    source: PathBuf,
    pages: PathBuf,
    dev: bool,
) -> Result<(), PageHandleError> {
    // dir is the root.
    let entries = rewrite_error(fs::read_dir(pages), File, Io, src)?;
    for entry in entries {
        if entry.as_ref().unwrap().path().is_dir() {
            let this = entry.unwrap().path();
            process_pages(&dir, &src, source.join(&this), this, dev)?;
        } else {
            let result = page(src, fs::read(entry.as_ref().unwrap().path()).unwrap(), dev);

            let path = dir.join("dist").join(
                entry
                    .unwrap()
                    .path()
                    .strip_prefix(src)
                    .unwrap()
                    .strip_prefix("pages")
                    .unwrap(),
            );

            rewrite_error(fs::create_dir_all(path.parent().unwrap()), File, Io, &path)?;
            let mut f = rewrite_error(std::fs::File::create(&path), File, Io, &path)?;
            rewrite_error(f.write(result?.as_bytes()), File, Io, &path)?;
            println!("With: {:?}", path);
        }
    }
    Ok(())
}

pub fn copy_into(public: &PathBuf, dist: &PathBuf) -> Result<(), Error> {
    if !dist.exists() {
        fs::create_dir_all(dist)?;
    }

    let entries = fs::read_dir(public)?;
    for entry in entries {
        let entry = entry.unwrap().path();
        let dest_path = dist.join(entry.strip_prefix(public).unwrap());

        if entry.is_dir() {
            copy_into(&entry, &dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&entry, &dest_path)?;
        }
    }
    Ok(())
}
