pub mod error;
pub mod reader;
pub mod types;
pub mod writer;

pub use error::ReanimError;
pub use reader::{decode, decode_pc, decode_phone32, decode_phone64};
pub use types::{Reanim, ReanimTrack, ReanimTransform, ReanimVersion};
pub use writer::encode;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_reanim() -> Reanim {
        let mut reanim = Reanim::default();
        reanim.fps = 60.0;

        let mut track = ReanimTrack {
            name: "TestTrack".to_string(),
            transforms: vec![],
        };

        let mut t1 = ReanimTransform::default();
        t1.x = Some(10.0);
        t1.y = Some(20.0);
        t1.font = Some("Arial".to_string());
        t1.text = Some("Hello".to_string());

        let mut t2 = ReanimTransform::default();
        t2.sx = Some(1.5);
        t2.sy = Some(1.5);
        t2.i = Some("123".to_string());

        track.transforms.push(t1);
        track.transforms.push(t2);

        reanim.tracks.push(track);
        reanim
    }

    #[test]
    fn test_reanim_roundtrip_pc() {
        let original = create_test_reanim();
        let encoded = encode(&original, ReanimVersion::PC).unwrap();
        let decoded = decode(&encoded).unwrap();

        assert_eq!(original.fps, decoded.fps);
        assert_eq!(original.tracks.len(), decoded.tracks.len());
        assert_eq!(original.tracks[0].name, decoded.tracks[0].name);
        assert_eq!(
            original.tracks[0].transforms[0].text,
            decoded.tracks[0].transforms[0].text
        );

        // `i` parameter in PC doesn't serialize as integer exclusively,
        // string roundtrips fine.
        assert_eq!(
            original.tracks[0].transforms[1].i,
            decoded.tracks[0].transforms[1].i
        );
    }

    #[test]
    fn test_reanim_roundtrip_phone32() {
        let original = create_test_reanim();
        let encoded = encode(&original, ReanimVersion::Phone32).unwrap();
        let decoded = decode(&encoded).unwrap();

        assert_eq!(original.fps, decoded.fps);
        assert_eq!(original.tracks.len(), decoded.tracks.len());
        assert_eq!(original.tracks[0].name, decoded.tracks[0].name);
        assert_eq!(
            original.tracks[0].transforms[0].text,
            decoded.tracks[0].transforms[0].text
        );
        assert_eq!(
            original.tracks[0].transforms[1].i,
            decoded.tracks[0].transforms[1].i
        );
    }

    #[test]
    fn test_reanim_roundtrip_phone64() {
        let original = create_test_reanim();
        let encoded = encode(&original, ReanimVersion::Phone64).unwrap();
        let decoded = decode(&encoded).unwrap();

        assert_eq!(original.fps, decoded.fps);
        assert_eq!(original.tracks.len(), decoded.tracks.len());
        assert_eq!(original.tracks[0].name, decoded.tracks[0].name);
        assert_eq!(
            original.tracks[0].transforms[0].text,
            decoded.tracks[0].transforms[0].text
        );
        assert_eq!(
            original.tracks[0].transforms[1].i,
            decoded.tracks[0].transforms[1].i
        );
    }
}
