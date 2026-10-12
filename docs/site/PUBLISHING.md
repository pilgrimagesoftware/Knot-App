# Publishing

GitHub repository settings must configure Pages to deploy from GitHub Actions. Pushes to `main` that change `docs/site` or `.github/workflows/docs-pages.yml` build and deploy the site.

Before merging, run `bundle install` and `bundle exec jekyll build` from `docs/site`. Confirm the generated root redirects to `latest`, and open each path listed in `_data/versions.yml`.
