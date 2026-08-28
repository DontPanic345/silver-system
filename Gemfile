source "https://rubygems.org"

# Pinned to the github-pages gem so local builds match GitHub's Pages
# build environment exactly (Jekyll + plugin versions included).
gem "github-pages", "~> 232", group: :jekyll_plugins

# Windows and JRuby platform helpers (harmless no-ops elsewhere)
platforms :mingw, :x64_mingw, :mswin, :jruby do
  gem "tzinfo", ">= 1", "< 3"
  gem "tzinfo-data"
end

gem "wdm", "~> 0.1.1", platforms: [:mingw, :x64_mingw, :mswin]

gem "http_parser.rb", "~> 0.6.0", platforms: [:jruby]
