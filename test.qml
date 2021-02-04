Window {
	Flex(Vertical) {
		Flex(Horizontal) {
			Baseline(20) {
				Text("Horizontal Flex")
			}
			Baseline(20) {
				Button[A]("Button A")
			}
		}
		ConstrainedBox(0..400, ..) {
			Form {
				Field("Field 1") {
					TextEdit {}
				}
				Field("Field 2") {
					TextEdit {}
				}
				Field("Slider") {
					Slider(.min = 0.0, .max=1.0) {
					}
				}
			}
		}
	}
}