export type OpenSourcePackage = {
	name: string;
	version?: string;
	license?: string;
	repository?: string;
	homepage?: string;
	description?: string;
	author?: string;
	licenseFile?: string;
};

export type OpenSourceSection = {
	id: string;
	label: string;
	packages: OpenSourcePackage[];
};

export type OpenSourceDocument = {
	generatedAt?: string;
	sections: OpenSourceSection[];
};
