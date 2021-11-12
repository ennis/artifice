Window {
    let mut text : String = Default::default();

	Flex(Vertical) {
		Flex(Horizontal) {
			Baseline(.value=20) {
				Text("Horizontal Flex")
			}
			Baseline(.value=20) {
				Button(.label="Button A")
			}
		}
		ConstrainedBox(0..400, ..) {
			Form {
				Field("Field 1") {
					TextEdit[text]
				}
				Field("Field 2") {
					TextEdit {}
				}
				Field("Slider") {
					Slider(.min = 0.0, .max=1.0)
				}
			}
		}
	}
}