# EncoderConfig に encoder_cfg (cfg_options_t) を追加する

Created: 2026-03-25
Model: Opus 4.6

## 概要

libaom の `aom_codec_enc_cfg_t` に含まれる `encoder_cfg` フィールド (`cfg_options_t`) が未実装。

## 根拠

EncoderConfig は libaom の `aom_codec_enc_cfg_t` のフィールドを網羅する方針だが、`cfg_options_t` は 35 フィールドを持つネストされた構造体であり、設計判断が必要。

## cfg_options_t のフィールド一覧

- `init_by_cfg_file`, `super_block_size`, `max_partition_size`, `min_partition_size`
- `disable_ab_partition_type`, `disable_rect_partition_type`, `disable_1to4_partition_type`
- `disable_flip_idtx`, `disable_cdef`, `disable_lr`, `disable_obmc`
- `disable_warp_motion`, `disable_global_motion`, `disable_dist_wtd_comp`
- `disable_diff_wtd_comp`, `disable_inter_intra_comp`, `disable_masked_comp`
- `disable_one_sided_comp`, `disable_palette`, `disable_intrabc`, `disable_cfl`
- `disable_smooth_intra`, `disable_filter_intra`, `disable_dual_filter`
- `disable_intra_angle_delta`, `disable_intra_edge_filter`, `disable_tx_64x64`
- `disable_smooth_inter_intra`, `disable_inter_inter_wedge`, `disable_inter_intra_wedge`
- `disable_paeth_intra`, `disable_trellis_quant`, `disable_ref_frame_mv`
- `reduced_reference_set`, `reduced_tx_type_set`

## pending の理由

1. `cfg_options_t` は libaom の設定ファイル (`--cfg` オプション) 経由で使用される内部構造体で、プログラム的に使うケースが少ない
2. 多くのフィールドは `AV1E_SET_*` 制御パラメータと機能が重複する (例: `disable_cdef` は `AV1E_SET_ENABLE_CDEF` と同等)
3. 35 フィールドの追加は EncoderConfig の肥大化につながるため、本当に必要か検討が必要

制御パラメータとの重複を整理した上で、独自に必要なフィールドのみ追加する方針が適切か判断する。
