import aabb_occlusion_culling as aac


def main():
    a = aac.PyOcclusionBuffer((0, 0), (4, 4))
    print(a.check_a_box(((5, 5), (6, 6))))


if __name__ == "__main__":
    main()
