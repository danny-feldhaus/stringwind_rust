#![feature(test)]
extern crate test;
extern crate image;
mod image_module;
mod string_setting;
mod path_generation;


#[cfg(test)]
mod tests{
    use super::string_setting::read_string_settings;
    use super::image_module::image_io::{read_lab, save_lab};
    use super::test::Bencher;
    use palette::{Lab, ColorDifference};
    use rand::Rng;

    #[test]
    fn generate_path()
    {

    }

    #[test]
    fn open_and_save_lab() 
    {
        let open_binder = read_lab("src/tests/images/vangogh.png");
        match open_binder
        {
            Ok(lab_img) =>
            {
                let save_binder = save_lab("src/tests/images/vangogh_resaved_lab.png", &lab_img);
                match save_binder 
                {
                    Ok(_s) => assert!(true),
                    Err(e) => 
                    {
                        println!("{:?}", e);
                        assert!(false);
                    }
                }
            },
            Err(e) => 
            {
                println!("{:?}", e);
                assert!(false);
            }
        };
    }

    #[test]
    fn read_settings()
    {
        let settings = read_string_settings("src/tests/settings");
        if settings.is_err()
        {
            println!("{:?}", settings.err());
        }
        else
        {
            let settings = settings.unwrap();
            let _str_cols =  settings.get::<Vec<Lab>>("str_colors").unwrap();
            let _bg_col = settings.get::<Lab>("bg_color").unwrap();
            let _in_path = settings.get::<String>("in_image_path").unwrap();
            let _out_image_path = settings.get::<String>("out_image_path").unwrap();
            let _pin_count = settings.get::<usize>("pin_count").unwrap();
            let _pin_radius = settings.get::<f32>("pin_radius").unwrap();
        }
        assert!(true);
    }

    #[test]
    #[should_panic]
    fn fail_read_settings()
    {
        let settings = read_string_settings("src/tests/settings").unwrap();
        let fail_test = settings.get::<usize>("fail_test");
        if(fail_test.is_err())
        {
            panic!("{:?}",fail_test.unwrap_err().to_string());
        }
    }

    #[bench]
    fn compare_lab_cielab(b: &mut Bencher)
    {
        let mut rng = rand::thread_rng();
        let rand_lab_a = Lab::new(
            rng.gen_range(0_f32..100_f32),
            rng.gen_range(-125_f32..125_f32),
            rng.gen_range(-125_f32..125_f32)
        );
        let rand_lab_b = Lab::new(
            rng.gen_range(0_f32..100_f32),
            rng.gen_range(-125_f32..125_f32),
            rng.gen_range(-125_f32..125_f32)
        );
        b.iter(||rand_lab_a.get_color_difference(&rand_lab_b));
    }

    #[bench]
    fn compare_lab_euclidean(b: &mut Bencher)
    {
        let mut rng = rand::thread_rng();
        let rand_lab_a = Lab::new(
            rng.gen_range(0_f32..100_f32),
            rng.gen_range(-125_f32..125_f32),
            rng.gen_range(-125_f32..125_f32)
        );
        let rand_lab_b = Lab::new(
            rng.gen_range(0_f32..100_f32),
            rng.gen_range(-125_f32..125_f32),
            rng.gen_range(-125_f32..125_f32)
        );
        b.iter(||
        {
            let diff = rand_lab_a - rand_lab_b;
            let _d = (diff.l*diff.l+diff.a*diff.a+diff.b*diff.b).sqrt();
        });
    }
}