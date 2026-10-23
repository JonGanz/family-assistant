# Family Assistant

## Claude's Job
I want to begin a project that ties together several elements in order to aid a busy family through the tedium of data blasts and organization that we currently get in today's age. There is a lot that I want added, so I'm going to break this down at a high level and your goal will be to architect a system and select appropriate technologies in order to satisfy these requirements. If there are any gaps in the requirements or questions to aid in the design, please prepare these in batches so that they can be answered. Do not let previous architectural paths influence potential changes based on responses.

### Process
I want you to take what has been outlined in this file and push us towards the goal by leveraging a directory structure and logging of decisions.

Note that we will use a lot of Markdown. Ensure the use of front-matter, as all the files described here will be managed by you, and you will need to ensure you can quickly find what you need. Feel free to add _index.md files that can point you in the correct direction.

Read through the entire document here before creating a CLAUDE.md file for yourself so that future sessions know what to do and the rules of work.

### Directory Structure
- create an /arch directory which will have one or more sets of architectural designs. For each design, there should be a markdown file that describes the architecture of that scope (for example, Overall.md might have a high-level design, while Frontend.md may have a more focused design for the front-end). There should also be a draw.io file for each markdown that renders the design using UML 2 formats. Whenever one is changed, the other must be changed as well.
- create a /design directory. Here you will create a markdown file for each question you have. There will be a spot for the answer. You will ask the questions in this format, by creating a file. I will answer them as I see fit in the alloted answers section, and you will be instructed to process those. When questions have been resolved, the files need to be moved
- create a /decisions directory. This is where you will catalogue decisions based on the answers to the questions.
- create a /log directory. This should be where you add daily logs of what was worked on and discussed.

### Self-Learn
As we work through this project, you are to self-improve. Leverage the directories above to look for commonalities and patterns that can be extracted into CLAUDE.md instructions or Skills, so that we can work faster and smarter the longer we work on this.

## Tech Goals
- everything should be able to run "locally". That means setting up local LLMs to handle any "AI" work.
    - target a system that is limited to 32GB of RAM with an 8GB RTX 3070.
- run as optimally as possible, as a local model will not be as fast or as capable, so we should do everything as determinisitcally as we can, pulling in LLMs for the other elements

## Project Goals
- the system needs to focus on the users, that is: the family members
    - members have roles: adult, child
    - members should be associated with their interests, hobbies, and responsibilities -- this will help guide the intelligent decision making by the system
- we want to be able to configure a list of email addresses (gmail), which the system can poll and process.
    - Each email can be filtered by the system and categorized
    - junk can be ignore
    - spam can be filtered for possible deals of interest
    - bills can be itemized
    - school information can be filtered to relevant details based on the age/grade/gender of the children
    - day care information can be similarly filtered; both should be associated with the correct child
    - emails from teachers should be linked to the appropriate child
- we want to configure and pull in calendars (caldev, google, etc), which can then be associated with individuals of the family
    - this is not for booking your work day, this is for personal life events (sports, school events or due dates, appointments, etc)
- the system should be able to pull details about relevant events from the emails that were surfaced, and have them added to the appropriate calendar if not already present
- we want to have the local weather pulled in and stored so we can pull that up at a glance
- tasks
    - the household as a whole needs tasks lists. There will be a large range.
    - house maintenance and cleaning schedules
    - homework and the steps to complete them
    - responding to emails
    - children's morning schedule (get dressed, eat, brush teeth, pack lunch, etc)
    - tasks can be auto-populate from emails, added from the interfaces, or be pushed into the "now important" list from a intelligent system
        - for example: clean the toilet once a week -- the system should track the last time it was cleaned, and add it to the list (perhaps always the same person, perhaps from a subset, for anyone to pick up, or intelligently assess the schedule -- all should be configurable)
    - tasks can be system managed based on time of year, weather, and how close the due date is
- food!
    - track the meal plan
    - input a link to the recipe, pull it down, extract the actual recipe, steps, and ingredients
    - create a grocery list given the upcoming meal plan up until a given date
- the ULTIMATE goal: simplify our lives by keeping track of all this information and taking off the mental load of tracking all of these things

## Interfacing
- one interface will be a Amazon Echo Show 15. We want a family dashboard page that anyone can view at a glance. This will require some hackery to keep open.
- mobile or desktop browser
- TUI (stretch objective)
