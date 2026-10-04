Gem::Specification.new do |spec|
  spec.name          = "rbs-sentinel"
  spec.version       = "0.7.0"
  spec.executables   = ["sentinel"]
  spec.bindir        = "bin"
  spec.authors       = ["Andrew Gauger"]
  spec.email         = ["andygauge@gmail.com"]
  spec.summary       = "Rust-powered RBS generator"
  spec.description   = "Sentinel scans Ruby files and generates RBS signatures using a high-performance Rust transpiler."
  spec.homepage      = "https://github.com/andygauge/rbs-sentinel"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.0"
  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = spec.homepage
  spec.files         = Dir["exe/*", "lib/**/*", "bin/*"]
end
