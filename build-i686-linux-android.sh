BASEDIR=$(dirname "$0")

export OPENSSL_LIB_DIR=`realpath "$BASEDIR"/deps/openssl`
export OPENSSL_INCLUDE_DIR=`realpath "$BASEDIR"/deps/openssl/include`

echo Compiling for i686-linux-android...
cargo ndk --target i686-linux-android --android-platform 28 -- build --release