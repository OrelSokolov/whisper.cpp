#!/usr/bin/env ruby
# frozen_string_literal: true

require 'fileutils'

# Build configuration
BUILD_DIR = 'build'
BUILD_TYPE = 'Release'
CMAKE = 'cmake'
MAKE = 'make'

namespace :build do
  desc "Configure CMake build system (Release mode)"
  task :configure do
    ensure_build_dir
    cmake_args = [
      "-B #{BUILD_DIR}",
      "-DCMAKE_BUILD_TYPE=#{BUILD_TYPE}"
    ]
    sh "#{CMAKE} #{cmake_args.join(' ')}"
  end

  desc "Build whisper-worker executable (Release mode)"
  task :worker do
    ensure_configured
    sh "#{CMAKE} --build #{BUILD_DIR} -j --config #{BUILD_TYPE} --target whisper-worker"
    puts "\n✓ Built: #{BUILD_DIR}/bin/whisper-worker"
  end

  desc "Build whisper-cli executable (Release mode)"
  task :cli do
    ensure_configured
    sh "#{CMAKE} --build #{BUILD_DIR} -j --config #{BUILD_TYPE} --target whisper-cli"
    puts "\n✓ Built: #{BUILD_DIR}/bin/whisper-cli"
  end

  namespace :rust do
    desc "Build whisper-worker-rs executable (Release mode)"
    task :worker do
      ensure_configured
      worker_rs_dir = 'examples/worker-rs'
      unless File.directory?(worker_rs_dir)
        puts "Error: #{worker_rs_dir} directory not found"
        exit 1
      end
      
      Dir.chdir(worker_rs_dir) do
        puts "Building Rust worker in #{worker_rs_dir}..."
        unless system('cargo build --release')
          puts "\n✗ Build failed"
          exit 1
        end
        puts "\n✓ Built: #{worker_rs_dir}/target/release/whisper-worker-rs"
      end
    end
  end

  desc "Build all examples and targets (Release mode)"
  task :all do
    ensure_configured
    sh "#{CMAKE} --build #{BUILD_DIR} -j --config #{BUILD_TYPE}"
    puts "\n✓ Built all targets in #{BUILD_DIR}/bin/"
  end

  desc "Clean build directory"
  task :clean do
    if File.directory?(BUILD_DIR)
      FileUtils.rm_rf(BUILD_DIR)
      puts "✓ Cleaned #{BUILD_DIR}/"
    else
      puts "Build directory #{BUILD_DIR}/ does not exist"
    end
  end
end

namespace :configure do
  desc "Configure build with Vulkan support (Release mode)"
  task :vulkan do
    ensure_build_dir
    cmake_args = [
      "-B #{BUILD_DIR}",
      "-DCMAKE_BUILD_TYPE=#{BUILD_TYPE}",
      "-DGGML_VULKAN=ON"
    ]
    sh "#{CMAKE} #{cmake_args.join(' ')}"
    puts "\n✓ Configured with Vulkan support (Release mode)"
    puts "  Run 'rake build:worker' or 'rake build:cli' to build"
  end

  desc "Configure build with NVIDIA CUDA support (Release mode)"
  task :nvidia do
    ensure_build_dir
    cmake_args = [
      "-B #{BUILD_DIR}",
      "-DCMAKE_BUILD_TYPE=#{BUILD_TYPE}",
      "-DGGML_CUDA=ON"
    ]
    sh "#{CMAKE} #{cmake_args.join(' ')}"
    puts "\n✓ Configured with NVIDIA CUDA support (Release mode)"
    puts "  Run 'rake build:worker' or 'rake build:cli' to build"
  end

  desc "Configure build with Metal support for macOS (Release mode)"
  task :metal do
    ensure_build_dir
    cmake_args = [
      "-B #{BUILD_DIR}",
      "-DCMAKE_BUILD_TYPE=#{BUILD_TYPE}",
      "-DGGML_METAL=ON"
    ]
    sh "#{CMAKE} #{cmake_args.join(' ')}"
    puts "\n✓ Configured with Metal support (Release mode)"
    puts "  Run 'rake build:worker' or 'rake build:cli' to build"
  end

  desc "Configure default build (CPU only, Release mode)"
  task :default do
    ensure_build_dir
    cmake_args = [
      "-B #{BUILD_DIR}",
      "-DCMAKE_BUILD_TYPE=#{BUILD_TYPE}"
    ]
    sh "#{CMAKE} #{cmake_args.join(' ')}"
    puts "\n✓ Configured default build (CPU only, Release mode)"
    puts "  Run 'rake build:worker' or 'rake build:cli' to build"
  end
end

namespace :spec do
  desc "Run all RSpec tests"
  task :all do
    sh 'bundle exec rspec'
  end

  desc "Run integration tests"
  task :integration do
    sh 'bundle exec rspec spec/integration'
  end

  desc "Run punctuation test"
  task :punctuation do
    sh 'bundle exec rspec spec/integration/punctuation_spec.rb'
  end

  desc "Run worker punctuation test"
  task :worker_punctuation do
    sh 'bundle exec rspec spec/integration/worker_punctuation_spec.rb'
  end

  desc "Run one worker punctuation test (single worker for all runs)"
  task :one_worker_punctuation do
    sh 'bundle exec rspec spec/integration/one_worker_punctuation_spec.rb'
  end
end

desc "Show available tasks"
task :help do
  puts <<~HELP
    Whisper.cpp Build Tasks
    ======================

    Build Tasks (all use Release mode by default):
      rake build:worker         - Build whisper-worker executable (C++)
      rake build:rust:worker   - Build whisper-worker-rs executable (Rust)
      rake build:cli            - Build whisper-cli executable
      rake build:all            - Build all examples and targets
      rake build:clean          - Clean build directory
      rake build:configure      - Configure CMake build system

    Configuration Tasks (all use Release mode by default):
      rake configure:vulkan - Configure with Vulkan GPU support
      rake configure:nvidia - Configure with NVIDIA CUDA support
      rake configure:metal  - Configure with Metal support (macOS only)
      rake configure:default - Configure default build (CPU only)

    Test Tasks:
      rake spec:all                    - Run all RSpec tests
      rake spec:integration            - Run integration tests
      rake spec:punctuation            - Run punctuation test (CLI)
      rake spec:worker_punctuation     - Run worker punctuation test (new worker per run)
      rake spec:one_worker_punctuation - Run worker punctuation test (single worker for all runs)

    Note: All builds use Release mode by default. Debug builds are not configured
    by default to ensure optimal performance and smaller binary sizes.

    Examples:
      rake configure:vulkan && rake build:worker
      rake configure:nvidia && rake build:cli
      rake build:rust:worker
      rake build:all
      rake spec:punctuation

  HELP
end

task :default => :help

def ensure_build_dir
  FileUtils.mkdir_p(BUILD_DIR) unless File.directory?(BUILD_DIR)
end

def ensure_configured
  unless File.exist?("#{BUILD_DIR}/CMakeCache.txt")
    puts "Build directory not configured. Running default configuration..."
    Rake::Task['build:configure'].invoke
  end
end

