//! Données ASCII pures (aucune logique). Masques : voir engine::mask_color.

pub const SUN: &str = r" \ | / 
- (O) -
 / | \ ";
pub const SUN_MASK: &str = r" y y y 
y yyy y
 y y y ";

pub const TREE_SMALL: &str = r"    .%&@&%.    
  .&@@@&@@@&.  
 %@@&@@@@@@@@% 
 &@@@@@@&@@@@& 
  '%&@@@@@&%'  
     \|#|/     
      |#|      
     /|#|\     
   _/_|#|_\_   ";
pub const TREE_SMALL_MASK: &str = r"    ggggggg    
  ggggggggggg  
 ggggggggggggg 
 ggggggggggggg 
  ggggggggggg  
     nnnnn     
      nnn      
     nnnnn     
   nnnnnnnnn   ";

pub const TREE_LARGE: &str = r"          .:%&@@&%:.           
       .%&@@@&&@@@@&%.         
     .&@@&%&@@@@@&&@@@&.       
   .%&@@@@@@&%&@@@@@@@@&%.     
  &@@@@@&@@@@@@@&@@@@&@@@@&    
 %@@@@@@@@@@@@@@@@@@@@@@@@@%   
.&@@@@@@&@@@@@@@@@@@&@@@@@@@&. 
%@@@@@@@@@@@@@@@@@@@@@@@@@@@@% 
&@@@@@&@@@@@@@@@@@@@@@@&@@@@@& 
%@@@@@@@@@@@&@@@@@@@@@@@@@@@@% 
 &@@@@@@@@@@@@@@@@@@@@@@@@@@&  
  '%&@@@@@@@@@@@@@@@@@@@@&%'   
     '%&@@@@@@&&@@@@@&%'       
           \  |||  /           
            \_|#|_/            
             |#=|              
             |#|#|             
             |=#|              
            /|#|#|\            
          _//|#=#|\\_          
       __/__/|#|#|\__\__       ";
pub const TREE_LARGE_MASK: &str = r"          gggggggggg           
       ggggggggggggggg         
     ggggggggggggggggggg       
   ggggggggggggggggggggggg     
  ggggggggggggggggggggggggg    
 ggggggggggggggggggggggggggg   
gggggggggggggggggggggggggggggg 
gggggggggggggggggggggggggggggg 
gggggggggggggggggggggggggggggg 
gggggggggggggggggggggggggggggg 
 gggggggggggggggggggggggggggg  
  gggggggggggggggggggggggggg   
     ggggggggggggggggggg       
           n  nnn  n           
            nnnnnnn            
             nnnn              
             nnnnn             
             nnnn              
            nnnnnnn            
          nnnnnnnnnnn          
       nnnnnnnnnnnnnnnnn       ";

/// Un motif de touffes par ligne de la bande d'herbe (répété sur la largeur).
pub const GRASS_ROWS: [&str; 4] = [
    "vWv,vw'vvW.wv,v",
    ",w'v.vW,v'wv,W.",
    "v.,'vv,w.'v,.v'",
    ",'.,v.',.,'v.,.",
];
pub const FLOWERS: [&str; 3] = ["*", "o", "@"];

pub const BIRD: [&str; 2] = [
r"  \ \ 
__( o>
      ",
r"      
__( o>
  / / ",
];
pub const BIRD_MASK: [&str; 2] = [
r"  w w 
www ky
      ",
r"      
www ky
  w w ",
];

pub const CLOUD_SMALL: &str = r"     .-~-.      
  .-(     )-.   
 (           )  
(             ) 
 `-._.--._.-~'  ";
pub const CLOUD_SMALL_MASK: &str = r"     wwwww      
  wwwwwwwwww    
 wwwwwwwwwwww   
wwwwwwwwwwwwww  
 cccccccccccc   ";

pub const CLOUD_LARGE: &str = r"        .-~~~-.         
     .-(       )-.      
   .(     .-~-.  )      
 .(     (      )   )-.  
(     (          )     )
(                      )
 `~-._.-~`~-._.-~`~-._-'";
pub const CLOUD_LARGE_MASK: &str = r"        wwwwwww         
     wwwwwwwwwww        
   wwwwwcccccwwww       
 wwwwwwcccccccwwwwww    
wwwwwwcccccccccwwwwwwww 
wwwwwwwwwwwwwwwwwwwwwwww
 ccccccccccccccccccccc  ";

pub const BUTTERFLY: [&str; 2] = ["><", "}{"];

// Contient des guillemets -> r#"..."#
pub const OWL: [&str; 2] = [
r#" ^ ^ 
(O,O)
(:v:)
 " " "#,
r#" ^ ^ 
(-,-)
(:v:)
 " " "#,
];
pub const OWL_MASK: [&str; 2] = [
r" n n 
nyyyn
nnwnn
 y y ",
r" n n 
nyyyn
nnwnn
 y y ",
];

pub const BEE: [&str; 2] = [">8<", "<8>"];
pub const APPLE: &str = "@";
pub const LEAF: &str = "%";

pub const RABBIT: [&str; 2] = [
r"      \\    
   __  \\   
  /  \__ o> 
  \(__) |_| ",
r"      //    
   __  //   
  /  \__ -> 
  \(__) |_| ",
];
pub const RABBIT_MASK: [&str; 2] = [
r"      mm    
       mm   
         km 
            ",
r"      mm    
       mm   
         km 
            ",
];

// Renard : corps orange (base rouge), queue en panache + poitrail blancs,
// bout des oreilles/truffe/pattes noirs. 2 frames = trot (pattes alternées).
pub const FOX: [&str; 2] = [
r"   /\        /\/\ 
  /  \______/  o \ 
  \          \_   >
   \_/\__/   ^   ^ ",
r"   /\        /\/\ 
  /  \______/  o \ 
  \          \_   >
   \_/\__/    ^ ^  ",
];
pub const FOX_MASK: [&str; 2] = [
r"   ww        knnk 
  ww rrrrrr r  k w 
  w          ww   k
   wwwwww    k   k ",
r"   ww        knnk 
  ww rrrrrr r  k w 
  w          ww   k
   wwwwww     k k  ",
];

pub const HEDGEHOG: &str = r#"  ,;;;;;;;,  
 ;;;;;;;;;(o>
  " "   " "  "#;
pub const HEDGEHOG_MASK: &str = r"  nnnnnnnnn  
 nnnnnnnnnwky
  n n   n n  ";

pub const MOUSE: &str = r#"      (\ 
~~(___o,>
   " "   "#;
pub const MOUSE_MASK: &str = r"      ww 
wwwwwwkwk
   w w   ";

pub const SNAIL: &str = "@_,";

// --- Nuit & météo ---

/// Croissant de lune : occupe le créneau du soleil (coin haut-droit).
pub const MOON: &str = r" .--
(   
 `--";
pub const MOON_MASK: &str = r" www
w   
 www";

/// Éclair : glyphe qui flashe brièvement dans le ciel (pas de flash plein écran).
pub const LIGHTNING: &str = r" _/
 / 
/_ 
 \ ";
pub const LIGHTNING_MASK: &str = r" yy
 y 
yy 
 y ";

pub const STAR_CHARS: [&str; 4] = [".", "*", "+", "'"]; // étoiles éparses dans le ciel nocturne
pub const RAIN_CHARS: [&str; 3] = ["|", "/", "\\"]; // vertical, incliné droite, incliné gauche
pub const FIREFLY: [&str; 2] = ["*", " "]; // clignote sur 2 frames
pub const WIND_CHARS: [&str; 3] = [",", "~", "'"]; // feuilles/débris emportés par le vent

#[cfg(test)]
mod tests {
    use super::*;

    // (frames, masks) à aligner char-pour-char. Chaque paire = un sprite masqué.
    fn masked_pairs() -> Vec<(Vec<&'static str>, Vec<&'static str>)> {
        vec![
            (vec![SUN], vec![SUN_MASK]),
            (vec![TREE_SMALL], vec![TREE_SMALL_MASK]),
            (vec![TREE_LARGE], vec![TREE_LARGE_MASK]),
            (vec![CLOUD_SMALL], vec![CLOUD_SMALL_MASK]),
            (vec![CLOUD_LARGE], vec![CLOUD_LARGE_MASK]),
            (BIRD.to_vec(), BIRD_MASK.to_vec()),
            (OWL.to_vec(), OWL_MASK.to_vec()),
            (RABBIT.to_vec(), RABBIT_MASK.to_vec()),
            (FOX.to_vec(), FOX_MASK.to_vec()),
            (vec![HEDGEHOG], vec![HEDGEHOG_MASK]),
            (vec![MOUSE], vec![MOUSE_MASK]),
            (vec![MOON], vec![MOON_MASK]),
            (vec![LIGHTNING], vec![LIGHTNING_MASK]),
        ]
    }

    #[test]
    fn every_mask_matches_its_frames() {
        let pairs = masked_pairs();
        assert!(pairs.len() >= 9);
        for (frames, masks) in pairs {
            assert_eq!(frames.len(), masks.len());
            for (f, m) in frames.iter().zip(masks.iter()) {
                let fl: Vec<&str> = f.split('\n').collect();
                let ml: Vec<&str> = m.split('\n').collect();
                assert_eq!(fl.len(), ml.len());
                for (a, b) in fl.iter().zip(ml.iter()) {
                    assert_eq!(a.chars().count(), b.chars().count(), "mask misaligned");
                }
            }
        }
    }

    #[test]
    fn bird_has_two_flap_frames() {
        assert_eq!(BIRD.len(), 2);
        assert_eq!(BIRD_MASK.len(), 2);
    }

    #[test]
    fn flowers_non_empty() {
        assert!(!FLOWERS.is_empty());
        assert!(FLOWERS.iter().all(|f| !f.is_empty()));
    }

    #[test]
    fn moon_and_lightning_are_multiline() {
        assert!(MOON.contains('\n'));
        assert!(LIGHTNING.contains('\n'));
    }

    #[test]
    fn single_glyph_char_sets() {
        assert!(STAR_CHARS.len() >= 2);
        assert!(STAR_CHARS.iter().all(|c| c.chars().count() == 1));
        assert!(RAIN_CHARS.iter().all(|c| c.chars().count() == 1));
        assert!(WIND_CHARS.iter().all(|c| c.chars().count() == 1));
    }

    #[test]
    fn firefly_blinks_over_two_frames() {
        assert_eq!(FIREFLY.len(), 2);
    }

    #[test]
    fn ground_animals_are_multiline_and_masked() {
        for (frames, masks) in [(RABBIT, RABBIT_MASK), (FOX, FOX_MASK)] {
            assert_eq!(frames.len(), 2);
            assert_eq!(masks.len(), 2);
            assert!(frames.iter().all(|f| f.split('\n').count() >= 3));
        }
        for sprite in [HEDGEHOG, MOUSE] {
            assert!(sprite.split('\n').count() >= 3);
        }
    }
}
