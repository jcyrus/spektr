class Spektr < Formula
  desc "Blazing-fast TUI utility for cleaning development artifacts"
  homepage "https://github.com/jcyrus/spektr"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/jcyrus/spektr/releases/download/v0.1.0/spektr-v0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "a3eefe2f53bf6ce3cdf4b350a99d4ec7e6e9488f7db98e0a28514719e93550e1"
    else
      url "https://github.com/jcyrus/spektr/releases/download/v0.1.0/spektr-v0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "bdc43805fe9b8ee4fc8f5fa7a882d7f147e2460ddeb2c933407cedb422d3e77c"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/jcyrus/spektr/releases/download/v0.1.0/spektr-v0.1.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "ea479c1981895e0843f62644457acb6fe848229190694951a7b297fd6bb09499"
    else
      url "https://github.com/jcyrus/spektr/releases/download/v0.1.0/spektr-v0.1.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "184722a9d14a7c8969a0f3bbf204386cd72d0c861636ce31016bcf3db2e9a142"
    end
  end

  def install
    bin.install "spektr"
  end

  test do
    assert_match "spektr", shell_output("#{bin}/spektr --help")
  end
end
