export PATH=$PATH:$HOME/.local/bin

maturin build --features  "extension-module" --release # actually build package
pip3 install --user --force-reinstall ./target/wheels/aabb_occlusion_culling-*.whl # install where it belongs
#stubgen -p aabb_occlusion_culling # generate updated stub files for next build to catch
#cp out/aabb_occlusion_culling/aabb_occlusion_culling.pyi .  # copy the generated stubs into work directory
#cargo doc --no-deps
