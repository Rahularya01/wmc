class Wmc < Formula
  desc "Clean downloaded WhatsApp media on macOS"
  homepage "https://github.com/Rahularya01/wmc"
  url "https://github.com/Rahularya01/wmc/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "1b95c4f7265e9cb0220a0d7b9532cc90688ac1b5c7f7a3c1712ec29638ee4d01"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "USAGE:", shell_output("#{bin}/wmc --help")
  end
end
