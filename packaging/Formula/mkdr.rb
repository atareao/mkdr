class Mkdr < Formula
  desc "TUI markdown renderer with paging"
  homepage "https://github.com/atareao/mkdr"
  version "0.3.3"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/atareao/mkdr/releases/download/v#{version}/mkdr-aarch64-apple-darwin.tar.gz"
      sha256 "SKIP"
    end
    on_intel do
      url "https://github.com/atareao/mkdr/releases/download/v#{version}/mkdr-x86_64-apple-darwin.tar.gz"
      sha256 "SKIP"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/atareao/mkdr/releases/download/v#{version}/mkdr-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "SKIP"
    end
    on_intel do
      url "https://github.com/atareao/mkdr/releases/download/v#{version}/mkdr-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "SKIP"
    end
  end

  def install
    bin.install "mkdr"
    bash_completion.install "completions/mkdr.bash" => "mkdr"
    zsh_completion.install "completions/mkdr.zsh" => "_mkdr"
    fish_completion.install "completions/mkdr.fish"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/mkdr --version")
  end
end