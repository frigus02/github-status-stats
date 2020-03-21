# Status Stats for GitHub

If you're using any CI system on your GitHub repository, chances are it pushes commit statuses to GitHub. The most recent ones are shown on the commit, as a :heavy_check_mark: tick or :x: cross.

GitHub stores a history of all commit statuses. If you retry a build on the same commit, it doesn't overwrite the previous status. It adds a new one. This gives us the ability to do some fun statistics. For example:

- Show builds with high/low success rate
- Show attempts needed for a build to pass
- Show build duration changes over time

We can use this to find flaky builds or prove builds got flakier or less flaky over time.

This is a GitHub app, which you can enable for your repositories. It collects data about statuses and check runs and gives you a dashoard so you can explore your build history.

**Important:** This is an experiment. It's not available for everyone, yet.

![](docs/preview.png)
