#!/usr/bin/env ruby
# frozen_string_literal: true

require 'xcodeproj'

project_path = 'AORATA.xcodeproj'
project = Xcodeproj::Project.open(project_path)

# Find the main target
target = project.targets.find { |t| t.name == 'AORATA' }
raise "AORATA target not found" unless target

puts "✅ Found target: #{target.name}"

# Add static libraries
lib_dir = File.expand_path('../build/universal', __dir__)
libraries = [
  'libcore_transport.a',
  'libcrypto_fabric.a',
  'libvfs_guard.a',
  'libedge_ai.a',
  'librelay_daemon.a'
]

libraries_group = project.main_group.find_subpath('Frameworks', true)

libraries.each do |lib_name|
  lib_path = File.join(lib_dir, lib_name)

  unless File.exist?(lib_path)
    puts "⚠️  Library not found: #{lib_path}"
    next
  end

  # Check if already added
  existing = target.frameworks_build_phase.files.find do |file|
    file.file_ref && file.file_ref.path && file.file_ref.path.end_with?(lib_name)
  end

  if existing
    puts "⏭️  Already linked: #{lib_name}"
    next
  end

  # Add library file reference
  file_ref = libraries_group.new_reference(lib_path)
  file_ref.source_tree = 'ABSOLUTE'

  # Add to frameworks build phase
  target.frameworks_build_phase.add_file_reference(file_ref)

  puts "✅ Linked: #{lib_name}"
end

# Add system frameworks
frameworks = [
  'Security',
  'SystemConfiguration'
]

frameworks.each do |framework_name|
  # Check if already added
  existing = target.frameworks_build_phase.files.find do |file|
    file.file_ref && file.file_ref.display_name == "#{framework_name}.framework"
  end

  if existing
    puts "⏭️  Already linked: #{framework_name}.framework"
    next
  end

  # Add framework
  framework = project.frameworks_group.new_reference("System/Library/Frameworks/#{framework_name}.framework")
  framework.source_tree = 'SDKROOT'
  target.frameworks_build_phase.add_file_reference(framework)

  puts "✅ Linked: #{framework_name}.framework"
end

# Set bridging header for all configurations
target.build_configurations.each do |config|
  # Set bridging header
  bridging_header = 'AORATA/Bridge/AORATACore-Bridging-Header.h'
  current_value = config.build_settings['SWIFT_OBJC_BRIDGING_HEADER']

  if current_value != bridging_header
    config.build_settings['SWIFT_OBJC_BRIDGING_HEADER'] = bridging_header
    puts "✅ Set bridging header for #{config.name}: #{bridging_header}"
  else
    puts "⏭️  Bridging header already set for #{config.name}"
  end

  # Ensure search paths include build directory
  header_search_paths = config.build_settings['HEADER_SEARCH_PATHS'] || []
  header_search_paths = [header_search_paths] unless header_search_paths.is_a?(Array)

  build_include = '$(PROJECT_DIR)/../build/include'
  unless header_search_paths.include?(build_include)
    header_search_paths << build_include
    config.build_settings['HEADER_SEARCH_PATHS'] = header_search_paths
    puts "✅ Added header search path for #{config.name}"
  end

  # Add library search paths
  library_search_paths = config.build_settings['LIBRARY_SEARCH_PATHS'] || []
  library_search_paths = [library_search_paths] unless library_search_paths.is_a?(Array)

  build_lib = '$(PROJECT_DIR)/../build/universal'
  unless library_search_paths.include?(build_lib)
    library_search_paths << build_lib
    config.build_settings['LIBRARY_SEARCH_PATHS'] = library_search_paths
    puts "✅ Added library search path for #{config.name}"
  end
end

# Save project
project.save

puts ""
puts "✅ Project updated successfully!"
puts ""
puts "Next steps:"
puts "1. Open AORATA.xcodeproj in Xcode"
puts "2. Build the project (Cmd+B)"
puts "3. Fix any remaining Swift errors"
puts ""
