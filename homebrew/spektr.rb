class Spektr < Formula
  desc "Blazing-fast TUI utility for cleaning development artifacts"
  homepage "https://github.com/jcyrus/spektr"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/jcyrus/spektr/releases/download/v0.1.0/spektr-0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_AARCH64"
    else
      url "https://github.com/jcyrus/spektr/releases/download/v0.1.0/spektr-0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_X86_64"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/jcyrus/spektr/releases/download/v0.1.0/spektr-0.1.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_LINUX_ARM"
    else
      url "https://github.com/jcyrus/spektr/releases/download/v0.1.0/spektr-0.1.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_LINUX_X86"
    end
  end

  def install
    bin.install "spektr"
  end

  test do
    assert_match "spektr", shell_output("#{bin}/spektr --help")
  end
end
