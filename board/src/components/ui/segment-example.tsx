import React from "react";
import { Segment, type SegmentOption } from "./segment";

// Example usage of the Segment component
export function SegmentExample() {
	const [selectedValue, setSelectedValue] = React.useState("option1");

	// Example 1: Basic segment with dots
	const basicOptions: SegmentOption[] = [
		{ value: "option1", label: "Option 1" },
		{ value: "option2", label: "Option 2" },
		{ value: "option3", label: "Option 3" },
	];

	// Example 2: Segment with custom icons
	const iconOptions: SegmentOption[] = [
		{ value: "home", label: "Home", icon: "🏠" },
		{ value: "search", label: "Search", icon: "🔍" },
		{ value: "settings", label: "Settings", icon: "⚙️" },
	];

	// Example 3: Segment without dots
	const noDotsOptions: SegmentOption[] = [
		{ value: "light", label: "Light" },
		{ value: "dark", label: "Dark" },
		{ value: "auto", label: "Auto" },
	];

	return (
		<div className="space-y-8 p-6">
			<div>
				<h3 className="text-lg font-semibold mb-4">Basic Segment with Dots</h3>
				<Segment
					options={basicOptions}
					value={selectedValue}
					onValueChange={setSelectedValue}
					showDots={true}
				/>
				<p className="text-sm text-gray-600 mt-2">Selected: {selectedValue}</p>
			</div>

			<div>
				<h3 className="text-lg font-semibold mb-4">Segment with Icons</h3>
				<Segment
					options={iconOptions}
					value="home"
					onValueChange={(value) => console.log("Selected:", value)}
					showDots={true}
				/>
			</div>

			<div>
				<h3 className="text-lg font-semibold mb-4">Segment without Dots</h3>
				<Segment
					options={noDotsOptions}
					value="light"
					onValueChange={(value) => console.log("Theme:", value)}
					showDots={false}
				/>
			</div>
		</div>
	);
}
