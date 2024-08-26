# Simple Build tool

A simple build tool that assembles a static site from components, templates, and data. I used it to build v6 of https://tnixc.space

- [ ] Fix problem where there are 2 sources of truth for md frontmatter in the current way I do blog posts, will hopefully be fixed with proper md rendering.
- [ ] Handle port collisions.
- [ ] Unindent markdown element
- [ ] Make it so that it fails more strictly on <Template>.
- [ ] Handle failure to parse json.
- [ ] Even better error messages that point out the exact line and file.
- [ ] Keep some elements loaded to avoid reading from disk every time.
- [ ] Switch to using a .config file
- [ ] Proper escaping from {}, "", etc. (?)
- [ ] Implement MD rendering (external lib) from .md files. Not needed tho maybe?
- [ ] Look into testing (tons of edge cases)
- [ ] OG Image and meta generation, especially for markdown posts
- [ ] Give warnings for unused and not found components
- [ ] CSS scoping in components but waiting for @scope general support , so just use tailwind for now
- [ ] Type safety-ish warnings when template doesn't match data
- [ ] Cache what has changed with hashing so no need to rebuild if stuff is same. -> can massively speed it up if lots of templating.
- [ ] Improve flexibility w/ markdown syntax highlighting theme
- [x] Check for circular deps.
- [x] Watcher or HMR (it was not as complicated as I thought, hell yeah)
- [x] Actually good logs with color, status, etc.
- [x] <markdown> component
- [x] Commands such as dev, build, new.
- [x] Props and slots
- [x] Make it so it ignores commented out lines.
- [x] The error handling is abysmal. Mainly due to me using unwrap(), Result type and ? everywhere. - more work to be done but it's in an ok state for now
- [x] Speed tests.
- [x] Get templates working
- [x] Get components working
- [x] Copy files from /public to /dist

documentation coming soon, once I get markdown stuff working?