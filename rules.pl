:- discontiguous(pass_image_layout/3).
:- discontiguous(image_format/2).
:- discontiguous(sequence/2).
:- discontiguous(image_size/3).
:- discontiguous(image_usage/2).
:- discontiguous(pass_resource_input_pipeline_stage/3).
:- discontiguous(pass_resource_output_pipeline_stage/3).
:- discontiguous(pass_resource_access_mask/3).

% declare image resources
image(a).
image(b).

% declare known image resource sizes
image_size(b, 1280, 720).

% declare passes
pass(p1).
pass(p2).
pass(p3).

% specifies required image usages (collected from all passes).
image_usage(a, transfer_dst).
image_usage(a, transfer_src).
image_usage(b, transfer_dst).

% specifies the layout of an image resource expected by the pass
pass_image_layout(p1, a, transfer_dst_optimal).
pass_image_layout(p2, a, transfer_read).
pass_image_layout(p2, b, transfer_write).
pass_image_layout(p3, b, present_src).

% specifies a set of possible image formats
% It is a list, with each entry representing a possible image format at runtime (format-set).
% There can be multiple formats sets. E.g. if there are two `image_format` facts, then there are two potentially
% valid format-sets for the image.
image_format_set(a, [rgba8, rg16f, rgba32f]).
image_format_set(b, X) :- image_format(a, X).  % same-format constraint

% specifies the image formats supported by a pass using an image. 
pass_supported_image_format(p2, a, r8).
pass_supported_image_format(p2, a, rg8).
pass_supported_image_format(p2, a, rgba8).
pass_supported_image_format(p2, a, r16f).
pass_supported_image_format(p2, a, rg16f).
pass_supported_image_format(p2, a, r32f).
pass_supported_image_format(p2, a, rg32f).
pass_supported_image_format(p2, a, rgba32f).

% specifies the pipeline stages that uses the resource for reading
pass_resource_input_pipeline_stage(p1, a, transfer). 
pass_resource_input_pipeline_stage(p2, a, transfer). 
pass_resource_input_pipeline_stage(p2, b, transfer). 

% specifies the pipeline stages that use the resource for writing
pass_resource_output_pipeline_stage(p1, a, transfer). 
pass_resource_output_pipeline_stage(p2, a, transfer). 
pass_resource_output_pipeline_stage(p2, b, transfer).

% pass resource access masks
pass_resource_access_mask(p1, a, transfer_write).
pass_resource_access_mask(p2, a, transfer_read).
pass_resource_access_mask(p2, b, transfer_write).

% defines a sequence relationship between two passes: a happens-before b
sequence(p1,p2).
sequence(p2,p3).



%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
compatible_image_format(ImageA, ImageB, Format) :- image_format(ImageA, Format), image_format(ImageB, Format).
compatible_image_formats(Img) :- 
    image_format(Img, PossibleFormats),
    forall(pass_supported_image_format(_, Img, Format), member(Format, PossibleFormats)).

sublist([],_).
sublist([X|Xs],Y) :- member(X,Y) , sublist(Xs,Y).

sequenced(X,Y) :- sequence(X,Y).
sequenced(X,Y) :- sequence(X,Z), sequenced(Z,Y).

