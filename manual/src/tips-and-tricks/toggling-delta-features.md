To toggle features such as `side-by-side` on and off, one solution is to use this shell function:

```sh
delta-toggle() {
    eval "export DELTA_FEATURES=$(-delta-features-toggle $1 | tee /dev/stderr)"
}
```
where `-delta-features-toggle` is this Python script:
[https://github.com/dandavison/tools/blob/main/python/-delta-features-toggle](https://github.com/dandavison/tools/blob/main/python/-delta-features-toggle).


Then

```
delta-toggle    # shows current features
delta-toggle s  # toggles side-by-side
delta-toggle l  # toggles line-numbers
```

(It might make sense to add something like this Python script to `delta` itself.)