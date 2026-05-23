class Mdr < Formula
  desc "TUI markdown renderer with paging"
  homepage "https://github.com/atareao/mdr"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/atareao/mdr/releases/download/v#{version}/mdr-aarch64-apple-darwin.tar.gz"
      sha256 "SKIP"
    end
    on_intel do
      url "https://github.com/atareao/mdr/releases/download/v#{version}/mdr-x86_64-apple-darwin.tar.gz"
      sha256 "SKIP"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/atareao/mdr/releases/download/v#{version}/mdr-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "SKIP"
    end
    on_intel do
      url "https://github.com/atareao/mdr/releases/download/v#{version}/mdr-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "SKIP"
    end
  end

  def install
    bin.install "mdr"
    bash_completion.install "completions/mdr.bash" => "mdr"
    zsh_completion.install "completions/mdr.zsh" => "_mdr"
    fish_completion.install "completions/mdr.fish"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/mdr --version")
  end
end
