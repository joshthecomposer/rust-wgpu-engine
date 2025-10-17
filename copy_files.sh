rm -rf ./target/debug/resources
rm -rf ./target/release/resources
rm -rf ./target/rwd/resources

mkdir -p ./target/debug/resources
mkdir -p ./target/debug/debug_out
mkdir -p ./target/debug/config

mkdir -p ./target/release/resources
mkdir -p ./target/release/debug_out
mkdir -p ./target/release/config

mkdir -p ./target/rwd/resources
mkdir -p ./target/rwd/debug_out
mkdir -p ./target/rwd/config

cp -r ./resources/. ./target/debug/resources
cp -r ./libs/. ./target/debug
cp -r ./debug_out/. ./target/debug/debug_out
cp -r ./config/. ./target/debug/config

cp -r ./resources/. ./target/release/resources
cp -r ./libs/. ./target/release
cp -r ./debug_out/. ./target/release/debug_out
cp -r ./config/. ./target/release/config

cp -r ./resources/. ./target/rwd/resources
cp -r ./libs/. ./target/rwd
cp -r ./debug_out/. ./target/rwd/debug_out
cp -r ./config/. ./target/rwd/config

# Cleanup some files:
rm -rf debug_out
mkdir debug_out
