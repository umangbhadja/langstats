#!/usr/bin/env ruby
require 'json'
require 'yaml'

abort 'usage: ruby scripts/update-linguist-metadata.rb LANGUAGES_YML' unless ARGV.length == 1
source = YAML.load_file(ARGV.fetch(0))
fields = %w[type group color extensions filenames aliases interpreters]
languages = source.sort.map do |name, properties|
  { 'name' => name }.merge(properties.select { |key, _| fields.include?(key) })
end
File.write(File.join(__dir__, '..', 'data', 'languages.json'), JSON.pretty_generate(languages) + "\n")
warn "generated #{languages.length} languages"
