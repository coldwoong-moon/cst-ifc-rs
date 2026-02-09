> **한국어** | [English](README.md)

# cst-ifc-rs

BIM/CAD 애플리케이션을 위한 고성능 Rust IFC(Industry Foundation Classes) 파서 및 메시 변환 라이브러리입니다.

## 주요 기능

- **스트리밍 IFC 파서**: 대용량 IFC 파일(400MB+ 테스트 완료)을 위한 메모리 효율적 STEP 텍스트 파싱
- **지오메트리 추출**: 색상/재질 지원이 포함된 IFCFACETEDBREP 삼각형 분할
- **메시 변환**: 정점 중복 제거를 통한 삼각형 메시 직접 변환
- **바이너리 내보내기**: 지오메트리 인스턴싱을 지원하는 컴팩트 바이너리 메시 포맷 (v3)
- **Three.js 연동**: 웹 기반 3D 렌더링을 위한 씬 내보내기

## 벤치마크

**테스트 파일**: 실제 콘크리트 건축물 IFC (Tekla Structures 내보내기)

| 항목 | 수치 |
|------|------|
| 파일 크기 | **395 MB** |
| 라인 수 | **4,847,558** |
| IFC 제품 | **276,593개** |
| 지오메트리 엔티티 | **3,714,855개** |

**성능** (Windows 11, Release 빌드, Rust 1.93.0):

| 작업 | 시간 | 처리량 |
|------|------|--------|
| 파싱 + 메시 변환 | **39.3초** | 10 MB/s, 123K lines/sec |
| 전체 파이프라인 (파싱 + 변환 + 내보내기) | **51.4초** | 730K triangles/sec |
| 바이너리 내보내기 | ~12초 | 848.5 MB 출력 |

**출력 결과**:
- 정점: **45,424,444개**
- 삼각형: **28,716,826개**
- 바이너리 메시 크기: **848.5 MB** (인스턴싱 포함 v3 포맷)
- 지오메트리 인스턴싱: **8개 중복 그룹** 탐지

**아키텍처**: 단일 스레드 STEP 파싱, 멀티스레드 메시 테셀레이션 (rayon)

## 타 라이브러리와 성능 비교

> **주의**: 아래 비교 데이터는 서로 다른 시스템과 IFC 파일에서 측정되었으므로 직접적인 비교에는 한계가 있습니다.
> 참고 목적으로만 활용해주세요.

### 파싱 + 지오메트리 처리 속도

| 라이브러리 | 언어 | 200MB IFC | 395MB IFC | 700MB IFC | 비고 |
|---|---|---|---|---|---|
| **cst-ifc-rs** | Rust | ~20s (추정) | **39.3s** (실측) | ~70s (추정) | 스트리밍 파서, 메모리 효율적 |
| **xBIM** | C#/.NET | 20s | ~33s (추정) | 46s | .NET 기반, 빠른 파싱 |
| **IfcOpenShell** | C++/Python | 70s | ~150s (추정) | 288s | OCC 기반 지오메트리 엔진 |
| **web-ifc** | C++/WASM | N/A | N/A | N/A | 55MB 이하만 테스트됨 |

*출처: [IfcOpenShell #6712](https://github.com/IfcOpenShell/IfcOpenShell/issues/6712), [web-ifc benchmark](https://github.com/ThatOpen/engine_web-ifc/blob/main/benchmark.md)*

### 소규모 파일 성능 (참고)

| 라이브러리 | 12MB IFC | 55MB IFC | 80MB IFC |
|---|---|---|---|
| **xBIM** | 2s | N/A | 3s |
| **web-ifc** | N/A | 5.9s (M1) | N/A |
| **IfcOpenShell** | 222s | N/A | 242s |

### 종합 비교

| 항목 | cst-ifc-rs | IfcOpenShell | xBIM | web-ifc |
|------|-----------|-------------|------|---------|
| **언어** | Rust | C++/Python | C#/.NET | C++/WASM |
| **라이선스** | MIT/Apache-2.0 | LGPL-3.0 | CDDL | MPL-2.0 |
| **대용량 파일** | 395MB 테스트 완료 | 700MB+ 지원 | 700MB+ 지원 | ~55MB 제한 |
| **메모리 모델** | 스트리밍 (상수 메모리) | 전체 로드 | 전체 로드 | WASM 힙 제한 |
| **지오메트리 엔진** | 자체 구현 | OpenCascade | 자체 구현 | 자체 구현 |
| **병렬 처리** | rayon (멀티스레드) | 단일 스레드 | 단일 스레드 | 단일 스레드 |
| **웹 지원** | 바이너리 내보내기 | 없음 | 없음 | 네이티브 WASM |
| **IFC 엔티티 지원** | FacetedBrep 중심 | 전체 IFC 스키마 | 전체 IFC 스키마 | 주요 엔티티 |
| **테스트** | 183개 | 광범위 | 광범위 | 기본 |

### cst-ifc-rs의 강점

1. **순수 Rust 구현**: 외부 C++ 라이브러리(OpenCascade 등) 의존 없음
2. **MIT/Apache-2.0 듀얼 라이선스**: 상용 프로젝트에 제한 없이 사용 가능
3. **스트리밍 파서**: 파일 크기에 관계없이 일정한 메모리 사용량
4. **멀티스레드 테셀레이션**: rayon 기반 병렬 메시 변환
5. **웹 최적화 출력**: Three.js 바이너리 포맷으로 직접 내보내기
6. **크로스 플랫폼**: Windows, macOS, Linux, WASM 타겟 지원

### 현재 제한사항

- IFC 엔티티 지원이 IFCFACETEDBREP 중심 (IFCEXTRUDEDAREASOLID 등 확장 필요)
- IFC 스키마 검증 기능 없음
- 속성(Property) 접근 API 미구현

## 테스트 결과

**183개 테스트 전체 통과**:

| 크레이트 | 테스트 수 | 설명 |
|---------|---------|------|
| cst-geometry | 47 | 커브, 서피스, NURBS, 테셀레이션 |
| cst-ifc | 70 (66 + 4) | STEP 파서, IFC 엔티티, 메시 변환 |
| cst-render | 22 | 카메라, 파이프라인, 씬, HTML 내보내기 |
| cst-mesh | 20 | 삼각형 메시, 어댑티브 테셀레이션 |
| cst-topology | 14 | 하프엣지, B-Rep 유효성 검증 |
| cst-math | 10 | 벡터, 트랜스폼, AABB, 레이 |

## 크레이트 구조

| 크레이트 | 설명 |
|---------|------|
| `cst-core` | 핵심 타입, 에러, 유틸리티 |
| `cst-math` | 수학 프리미티브 (glam 기반): 벡터, 행렬, 변환 |
| `cst-topology` | B-Rep 토폴로지: 하프엣지 데이터 구조 (slotmap 아레나) |
| `cst-geometry` | 커브, 서피스, NURBS 평가 |
| `cst-mesh` | B-Rep에서 삼각형 메시 테셀레이션 |
| `cst-ifc` | IFC/STEP 파서 및 엔티티 매핑 |
| `cst-render` | 씬 관리 및 바이너리 메시 내보내기 |

## 빠른 시작

```rust
use cst_ifc::ifc_reader;
use cst_ifc::ifc_to_mesh;
use cst_mesh::TriangleMesh;
use cst_render::Scene;

// IFC 파일 파싱 (스트리밍, 상수 메모리)
let ifc_data = ifc_reader::read_ifc_file("model.ifc".as_ref())?;

// 지오메트리를 삼각형 메시로 변환
let mut scene = Scene::new();
for mesh_data in &ifc_data {
    let trimesh = ifc_to_mesh::faces_to_trimesh(&mesh_data.name, &mesh_data.faces);

    if trimesh.triangle_count() > 0 {
        let mesh = TriangleMesh {
            positions: trimesh.positions,
            normals: trimesh.normals,
            indices: trimesh.indices,
            uvs: vec![],
        };
        let color = mesh_data.color.unwrap_or([0.7, 0.7, 0.7]);
        scene.add_mesh(&mesh_data.name, mesh, color);
    }
}

// 인터랙티브 HTML 뷰어로 내보내기
scene.export_html("output.html".as_ref())?;
```

### CLI 도구

```bash
# IFC를 바이너리 메시로 변환
cargo run --release --bin ifc_viewer -- input.ifc output.html

# 테스트 실행
cargo test --release
```

## 바이너리 메시 포맷 (v3)

지오메트리 인스턴싱을 지원하는 효율적인 바이너리 포맷:

```
[u8 version=3]
[u32 regular_mesh_count]
[u32 instanced_group_count]

일반 메시 (각각):
  [u32 name_len][name_bytes]
  [f32 r][f32 g][f32 b]
  [u32 vertex_count][u32 index_count]
  [vertex_count × 3 × f32 positions]
  [index_count × u32 indices]

인스턴스 그룹 (각각):
  [u32 name_len][name_bytes]
  [f32 r][f32 g][f32 b]
  [u32 vertex_count][u32 index_count][u32 instance_count]
  [vertex_count × 3 × f32 positions]
  [index_count × u32 indices]
  [instance_count × 16 × f32 transform_matrices (4×4 column-major)]
```

**특징**:
- 자동 중복 지오메트리 탐지
- 반복 요소에 대한 변환 기반 인스턴싱
- 일반적인 BIM 모델에서 ~40-60% 크기 절감

## 의존성

모든 의존성은 MIT/Apache-2.0 라이선스:
- glam 0.29 (수학 프리미티브)
- nalgebra 0.33 (선형대수)
- slotmap 1.0 (아레나 할당)
- rayon 1.10 (병렬 처리)
- serde 1.0 + bincode 1.0 (직렬화)

## 관련 프로젝트

- **[cst-web-viewer](https://github.com/coldwoong-moon/cst-web-viewer)** - Three.js 기반 웹 3D 뷰어 (Tekla Structures 스타일 네비게이션)

## 라이선스

MIT 또는 Apache-2.0 듀얼 라이선스.
