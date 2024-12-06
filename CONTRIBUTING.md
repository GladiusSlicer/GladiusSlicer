# Contributing
## Found an issue?
One way you can help is to report bugs on our GitHub issue tracker. Please include a copy of your model, settings files, and command output, so we can reproduce your issue. If the issue is with your GCODE please include that as well.

Before posting your issue, please take a moment to search the tracker's existing issues first, as it's possible that someone else reported the same issue before you. Though it helps save time, don't worry! We won't mind if you accidentally post a duplicate issue.

We will attempt to fix your issue for the next release.

## Feature Requests

If there is a feature you think would be great to add, please let us know by submitting a ticket to the Github issue tracker. Please note, GUI requests/changes are best for the GUI repo.

## Pull Requests

So, you want to write some code? Great!

To begin helping out, fork the repository to your account and `git clone` the forked
copy to your local machine. On clone you will be on the *master* branch. This
is the branch that contains all new work that has not been released yet. If you
are adding a new feature then you want to base your work off of this branch.

Contributors should be familiar with the [Git Style Guide](https://github.com/agis/git-style-guide) and [Commit Message Guidelines](https://gist.github.com/robertpainsi/b632364184e70900af4ab688decf6f53).

### Submission Checklist

Before submitting your pull request to the repository, please make sure you have
done the following things first:

1. You have ensured the pull request is rebased on a recent version of your
   respective branch or the latest upstream has been merged.

1. All of the following commands completed without warnings or errors.  CI will test with all features enabled.
   - `cargo fmt --all`
   - `cargo clippy`
   - `cargo run --release`


## Open Areas of development
* Adding MacOS support
* Remove Bindgen / Clipper as a dependency
* Adding Support Material
* Improve documentation
