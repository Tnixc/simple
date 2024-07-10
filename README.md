# Simple Build tool
- [ ] Check for circular deps
- [ ] Implement MD rendering (external lib)
- [ ] Look into testing
- [ ] Give warnings for unused and not found components
- [ ] Watcher or HMR (HMR is too complex so probably not)
- [ ] CSS scoping in components but waiting for [@scope general support](https://developer.mozilla.org/en-US/docs/Web/CSS/@scope)
- [ ] Type safety warnings when template doesn't match data
- [ ] Speed tests.
- [ ] Cache what has changed with hashing so no need to rebuild if stuff is same. -> can massively speed it up if lots of templating.
- [x] Get templates working
- [x] Get components working
- [x] Copy files from /public to /dist

---

It's very little code. Basically a glorified build script but I think it's pretty neat.

<details>
<summary>Random notes</summary>
![in Templates](https://github.com/Tnixc/simple/assets/85466117/e90a0455-320b-4d37-8ad2-2efd265171e3)
</details>
